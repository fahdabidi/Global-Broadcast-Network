use gbn_bridge_protocol::{
    BootstrapDhtEntry, BootstrapProgressStage, BridgeAck, BridgeCapability, BridgeClose,
    BridgeData, BridgeIngressEndpoint, BridgeLease, BridgeOpen, BridgePunchAck, BridgePunchProbe,
    BridgePunchStart, BridgeRegister, BridgeSetRequest, BridgeSetResponse, ReachabilityClass,
};
use gbn_bridge_publisher::{AuthorityBootstrapPlan, AuthorityError};

use crate::bootstrap_bridge::BootstrapBridgeState;
use crate::creator_listener::CreatorListener;
use crate::forwarder::PayloadForwarder;
use crate::heartbeat_loop::HeartbeatLoop;
use crate::lease_state::LeaseState;
use crate::progress_reporter::ProgressReporter;
use crate::publisher_client::InProcessPublisherClient;
use crate::punch::{PunchManager, PunchSource};
use crate::session::BridgeSessionRegistry;
use crate::{RuntimeError, RuntimeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExitBridgeConfig {
    pub bridge_id: String,
    pub identity_pub: gbn_bridge_protocol::PublicKeyBytes,
    pub ingress_endpoint: BridgeIngressEndpoint,
    pub requested_udp_punch_port: u16,
    pub capabilities: Vec<BridgeCapability>,
}

impl ExitBridgeConfig {
    pub fn registration_request(&self) -> BridgeRegister {
        BridgeRegister {
            bridge_id: self.bridge_id.clone(),
            identity_pub: self.identity_pub.clone(),
            ingress_endpoints: vec![self.ingress_endpoint.clone()],
            requested_udp_punch_port: self.requested_udp_punch_port,
            capabilities: self.capabilities.clone(),
        }
    }
}

#[derive(Debug)]
pub struct ExitBridgeRuntime {
    config: ExitBridgeConfig,
    publisher_client: InProcessPublisherClient,
    lease_state: LeaseState,
    creator_listener: CreatorListener,
    heartbeat_loop: HeartbeatLoop,
    bootstrap_bridge: BootstrapBridgeState,
    punch_manager: PunchManager,
    progress_reporter: ProgressReporter,
    forwarder: PayloadForwarder,
    data_sessions: BridgeSessionRegistry,
    registered_reachability_class: Option<ReachabilityClass>,
}

impl ExitBridgeRuntime {
    pub fn new(config: ExitBridgeConfig, publisher_client: InProcessPublisherClient) -> Self {
        Self {
            config,
            publisher_client,
            lease_state: LeaseState::default(),
            creator_listener: CreatorListener::default(),
            heartbeat_loop: HeartbeatLoop,
            bootstrap_bridge: BootstrapBridgeState::default(),
            punch_manager: PunchManager::default(),
            progress_reporter: ProgressReporter::default(),
            forwarder: PayloadForwarder::default(),
            data_sessions: BridgeSessionRegistry::default(),
            registered_reachability_class: None,
        }
    }

    pub fn config(&self) -> &ExitBridgeConfig {
        &self.config
    }

    pub fn publisher_client(&self) -> &InProcessPublisherClient {
        &self.publisher_client
    }

    pub fn publisher_client_mut(&mut self) -> &mut InProcessPublisherClient {
        &mut self.publisher_client
    }

    pub fn lease_state(&self) -> &LeaseState {
        &self.lease_state
    }

    pub fn current_lease(&self) -> Option<&BridgeLease> {
        self.lease_state.current()
    }

    pub fn ingress_is_exposed(&mut self, now_ms: u64) -> bool {
        self.refresh_ingress(now_ms);
        self.creator_listener.is_exposed()
    }

    pub fn local_progress_events(&self) -> &[gbn_bridge_protocol::BootstrapProgress] {
        self.progress_reporter.emitted()
    }

    pub fn local_forwarded_frames(&self) -> &[crate::forwarder::ForwardedFrame] {
        self.forwarder.forwarded()
    }

    pub fn active_data_session_count(&self) -> usize {
        self.data_sessions.active_session_count()
    }

    pub fn startup(
        &mut self,
        reachability_class: ReachabilityClass,
        now_ms: u64,
    ) -> RuntimeResult<BridgeLease> {
        self.registered_reachability_class = Some(reachability_class.clone());
        let lease = self.publisher_client.register_bridge(
            self.config.registration_request(),
            reachability_class,
            now_ms,
        )?;
        self.apply_lease(lease.clone(), now_ms);
        Ok(lease)
    }

    pub fn heartbeat_tick(
        &mut self,
        active_sessions: u32,
        now_ms: u64,
    ) -> RuntimeResult<Option<BridgeLease>> {
        let Some(heartbeat) = self.heartbeat_loop.maybe_build_heartbeat(
            &self.lease_state,
            &self.config.bridge_id,
            active_sessions,
            now_ms,
        ) else {
            self.refresh_ingress(now_ms);
            return Ok(None);
        };

        match self.publisher_client.renew_lease(heartbeat) {
            Ok(lease) => {
                self.lease_state.mark_heartbeat_sent(now_ms);
                self.apply_lease(lease.clone(), now_ms);
                Ok(Some(lease))
            }
            Err(AuthorityError::BridgeNotFound { .. }) => {
                let reachability_class =
                    self.registered_reachability_class.clone().ok_or_else(|| {
                        RuntimeError::MissingReachabilityClass {
                            bridge_id: self.config.bridge_id.clone(),
                        }
                    })?;
                let lease = self.publisher_client.register_bridge(
                    self.config.registration_request(),
                    reachability_class,
                    now_ms,
                )?;
                self.lease_state.mark_heartbeat_sent(now_ms);
                self.apply_lease(lease.clone(), now_ms);
                Ok(Some(lease))
            }
            Err(err @ AuthorityError::BridgeRevoked { .. })
            | Err(err @ AuthorityError::LeaseExpired { .. }) => {
                self.lease_state.clear();
                self.creator_listener.disable(now_ms);
                Err(err.into())
            }
            Err(err) => Err(err.into()),
        }
    }

    pub fn apply_seed_assignment(
        &mut self,
        plan: &AuthorityBootstrapPlan,
        now_ms: u64,
    ) -> RuntimeResult<bool> {
        self.bootstrap_bridge.assign_from_plan(
            &self.config.bridge_id,
            &self.publisher_client.publisher_public_key(),
            plan,
            now_ms,
        )
    }

    pub fn begin_publisher_directed_punch(
        &mut self,
        instruction: BridgePunchStart,
        now_ms: u64,
    ) -> RuntimeResult<BridgePunchProbe> {
        let lease = self.require_direct_ingress(now_ms)?;
        self.punch_manager.begin_from_instruction(
            &self.punch_source(lease.udp_punch_port),
            &self.publisher_client.publisher_public_key(),
            instruction,
            now_ms,
        )
    }

    pub fn begin_refresh_punch(
        &mut self,
        target: BootstrapDhtEntry,
        now_ms: u64,
    ) -> RuntimeResult<BridgePunchProbe> {
        let lease = self.require_direct_ingress(now_ms)?;
        self.punch_manager.begin_from_refresh_entry(
            &self.punch_source(lease.udp_punch_port),
            &self.publisher_client.publisher_public_key(),
            target,
            now_ms,
        )
    }

    pub fn acknowledge_tunnel(
        &mut self,
        bootstrap_session_id: &str,
        responder_node_id: &str,
        observed_udp_punch_port: u16,
        acked_probe_nonce: u64,
        established_at_ms: u64,
    ) -> RuntimeResult<BridgePunchAck> {
        let ack = self.punch_manager.acknowledge(
            bootstrap_session_id,
            &self.config.bridge_id,
            responder_node_id,
            observed_udp_punch_port,
            acked_probe_nonce,
            established_at_ms,
        )?;

        let stage = if self.bootstrap_bridge.has_assignment(bootstrap_session_id) {
            BootstrapProgressStage::SeedTunnelEstablished
        } else {
            BootstrapProgressStage::BridgeTunnelEstablished
        };
        self.progress_reporter.report(
            &mut self.publisher_client,
            &self.config.bridge_id,
            bootstrap_session_id,
            stage,
            1,
            established_at_ms,
        );

        Ok(ack)
    }

    pub fn serve_bridge_set(
        &mut self,
        request: &BridgeSetRequest,
        now_ms: u64,
    ) -> RuntimeResult<BridgeSetResponse> {
        let response = self.bootstrap_bridge.serve_bridge_set(
            request,
            &self.publisher_client.publisher_public_key(),
            now_ms,
        )?;
        self.progress_reporter.report(
            &mut self.publisher_client,
            &self.config.bridge_id,
            &request.bootstrap_session_id,
            BootstrapProgressStage::SeedPayloadReceived,
            response.bridge_entries.len() as u16,
            now_ms,
        );
        Ok(response)
    }

    pub fn forward_creator_frame(&mut self, frame: BridgeData, now_ms: u64) -> RuntimeResult<()> {
        let _lease = self.require_direct_ingress(now_ms)?;
        self.forwarder.forward(&mut self.publisher_client, frame);
        Ok(())
    }

    pub fn open_data_session(&mut self, open: BridgeOpen, now_ms: u64) -> RuntimeResult<()> {
        let _lease = self.require_direct_ingress(now_ms)?;
        if open.bridge_id != self.config.bridge_id {
            return Err(RuntimeError::UnexpectedBridgeRuntime {
                expected_bridge_id: self.config.bridge_id.clone(),
                actual_bridge_id: open.bridge_id,
            });
        }

        self.publisher_client.open_bridge_session(open.clone())?;
        self.data_sessions.open(open);
        Ok(())
    }

    pub fn forward_session_frame(
        &mut self,
        frame: BridgeData,
        now_ms: u64,
    ) -> RuntimeResult<BridgeAck> {
        let _lease = self.require_direct_ingress(now_ms)?;
        self.data_sessions.require_session(&frame.session_id)?;
        self.forwarder
            .forward(&mut self.publisher_client, frame.clone());
        self.publisher_client
            .ingest_bridge_frame(&self.config.bridge_id, frame, now_ms)
            .map_err(Into::into)
    }

    pub fn close_data_session(&mut self, close: BridgeClose, now_ms: u64) -> RuntimeResult<()> {
        let _lease = self.require_direct_ingress(now_ms)?;
        self.data_sessions.close(&close)?;
        self.publisher_client.close_bridge_session(close)?;
        Ok(())
    }

    fn apply_lease(&mut self, lease: BridgeLease, now_ms: u64) {
        self.lease_state.update_lease(lease, now_ms);
        self.refresh_ingress(now_ms);
    }

    fn refresh_ingress(&mut self, now_ms: u64) {
        self.creator_listener
            .refresh_from_lease(&self.lease_state, now_ms);
    }

    fn require_direct_ingress(&mut self, now_ms: u64) -> RuntimeResult<BridgeLease> {
        self.refresh_ingress(now_ms);
        let lease =
            self.lease_state
                .current_cloned()
                .ok_or_else(|| RuntimeError::NoActiveLease {
                    bridge_id: self.config.bridge_id.clone(),
                })?;

        if !matches!(lease.reachability_class, ReachabilityClass::Direct) {
            return Err(RuntimeError::NonDirectReachability {
                bridge_id: self.config.bridge_id.clone(),
                reachability_class: lease.reachability_class,
            });
        }

        if !self.creator_listener.is_exposed() {
            return Err(RuntimeError::IngressDisabled {
                bridge_id: self.config.bridge_id.clone(),
            });
        }

        Ok(lease)
    }

    fn punch_source(&self, source_udp_punch_port: u16) -> PunchSource {
        PunchSource {
            bridge_id: self.config.bridge_id.clone(),
            bridge_identity_pub: self.config.identity_pub.clone(),
            source_ip_addr: self.config.ingress_endpoint.host.clone(),
            source_udp_punch_port,
        }
    }
}
