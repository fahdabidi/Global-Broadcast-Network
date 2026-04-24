use std::collections::BTreeMap;

use gbn_bridge_protocol::{
    BootstrapDhtEntry, BootstrapProgressStage, BridgeAck, BridgeBatchAssign, BridgeCapability,
    BridgeCatalogResponse, BridgeClose, BridgeCommandAckStatus, BridgeCommandPayload,
    BridgeControlCommand, BridgeData, BridgeIngressEndpoint, BridgeLease, BridgeOpen,
    BridgePunchAck, BridgePunchProbe, BridgePunchStart, BridgeRegister, BridgeRevoke,
    BridgeSetRequest, BridgeSetResponse, ReachabilityClass,
};
use gbn_bridge_publisher::{AuthorityBootstrapPlan, AuthorityError};

use crate::bootstrap_bridge::BootstrapBridgeState;
use crate::control_client::BridgeControlClient;
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
    control_client: Option<BridgeControlClient>,
    bootstrap_chain_ids: BTreeMap<String, String>,
    pending_batch_assignments: Vec<BridgeBatchAssign>,
    received_catalog_refreshes: Vec<BridgeCatalogResponse>,
    last_revocation: Option<BridgeRevoke>,
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
            control_client: None,
            bootstrap_chain_ids: BTreeMap::new(),
            pending_batch_assignments: Vec::new(),
            received_catalog_refreshes: Vec::new(),
            last_revocation: None,
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

    pub fn attach_control_client(&mut self, client: BridgeControlClient) {
        self.control_client = Some(client);
    }

    pub fn control_client(&self) -> Option<&BridgeControlClient> {
        self.control_client.as_ref()
    }

    pub fn pending_batch_assignments(&self) -> &[BridgeBatchAssign] {
        &self.pending_batch_assignments
    }

    pub fn received_catalog_refreshes(&self) -> &[BridgeCatalogResponse] {
        &self.received_catalog_refreshes
    }

    pub fn last_revocation(&self) -> Option<&BridgeRevoke> {
        self.last_revocation.as_ref()
    }

    pub fn lease_state(&self) -> &LeaseState {
        &self.lease_state
    }

    pub fn current_lease(&self) -> Option<&BridgeLease> {
        self.lease_state.current()
    }

    pub fn apply_remote_lease(&mut self, lease: BridgeLease, now_ms: u64) {
        self.apply_lease(lease, now_ms);
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

    pub fn active_punch_attempt(
        &self,
        bootstrap_session_id: &str,
    ) -> Option<&crate::punch::ActivePunchAttempt> {
        self.punch_manager.active_attempt(bootstrap_session_id)
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
        if self.control_client.is_some() {
            self.report_progress_control_path(bootstrap_session_id, stage, 1, established_at_ms)?;
        } else {
            self.progress_reporter.report(
                &mut self.publisher_client,
                &self.config.bridge_id,
                bootstrap_session_id,
                stage,
                1,
                established_at_ms,
            );
        }

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
        if self.control_client.is_some() {
            self.report_progress_control_path(
                &request.bootstrap_session_id,
                BootstrapProgressStage::SeedPayloadReceived,
                response.bridge_entries.len() as u16,
                now_ms,
            )?;
        } else {
            self.progress_reporter.report(
                &mut self.publisher_client,
                &self.config.bridge_id,
                &request.bootstrap_session_id,
                BootstrapProgressStage::SeedPayloadReceived,
                response.bridge_entries.len() as u16,
                now_ms,
            );
        }
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

    pub fn receive_next_control_command(
        &mut self,
        now_ms: u64,
    ) -> RuntimeResult<Option<gbn_bridge_protocol::BridgeCommandAck>> {
        if self.control_client.is_none() {
            return Err(RuntimeError::MissingControlClient);
        }
        let command = {
            let client = self
                .control_client
                .as_mut()
                .expect("control client should exist");
            client.receive_command(now_ms)?
        };
        let Some(command) = command else {
            return Ok(None);
        };
        let status = self.apply_control_command(&command, now_ms)?;
        let ack = self
            .control_client
            .as_mut()
            .expect("control client should exist")
            .acknowledge_command(&command, status, now_ms)?;
        Ok(Some(ack))
    }

    pub fn send_control_keepalive(&mut self, now_ms: u64) -> RuntimeResult<()> {
        let Some(client) = self.control_client.as_mut() else {
            return Err(RuntimeError::MissingControlClient);
        };
        client.send_keepalive(now_ms)
    }

    fn apply_control_command(
        &mut self,
        command: &BridgeControlCommand,
        now_ms: u64,
    ) -> RuntimeResult<BridgeCommandAckStatus> {
        match &command.payload {
            BridgeCommandPayload::PunchStart(payload) => {
                self.bootstrap_chain_ids.insert(
                    payload.bootstrap_session_id.clone(),
                    command.chain_id.clone(),
                );
                self.begin_publisher_directed_punch(payload.clone(), now_ms)?;
                Ok(BridgeCommandAckStatus::Applied)
            }
            BridgeCommandPayload::BatchAssign(payload) => {
                if payload.bridge_id != self.config.bridge_id {
                    return Err(RuntimeError::UnexpectedBridgeRuntime {
                        expected_bridge_id: self.config.bridge_id.clone(),
                        actual_bridge_id: payload.bridge_id.clone(),
                    });
                }
                for assignment in &payload.assignments {
                    self.bootstrap_chain_ids.insert(
                        assignment.bootstrap_session_id.clone(),
                        command.chain_id.clone(),
                    );
                }
                self.pending_batch_assignments.push(payload.clone());
                Ok(BridgeCommandAckStatus::Applied)
            }
            BridgeCommandPayload::Revoke(payload) => {
                if payload.bridge_id != self.config.bridge_id {
                    return Err(RuntimeError::UnexpectedBridgeRuntime {
                        expected_bridge_id: self.config.bridge_id.clone(),
                        actual_bridge_id: payload.bridge_id.clone(),
                    });
                }
                self.lease_state.clear();
                self.creator_listener.disable(now_ms);
                self.last_revocation = Some(payload.clone());
                Ok(BridgeCommandAckStatus::Applied)
            }
            BridgeCommandPayload::CatalogRefresh(payload) => {
                self.received_catalog_refreshes.push(payload.clone());
                Ok(BridgeCommandAckStatus::Applied)
            }
        }
    }

    fn report_progress_control_path(
        &mut self,
        bootstrap_session_id: &str,
        stage: BootstrapProgressStage,
        active_bridge_count: u16,
        reported_at_ms: u64,
    ) -> RuntimeResult<()> {
        let Some(chain_id) = self.bootstrap_chain_ids.get(bootstrap_session_id).cloned() else {
            return Ok(());
        };
        let Some(control_client) = self.control_client.as_mut() else {
            return Ok(());
        };
        control_client.send_progress(
            &chain_id,
            gbn_bridge_protocol::BootstrapProgress {
                bootstrap_session_id: bootstrap_session_id.to_string(),
                reporter_id: self.config.bridge_id.clone(),
                stage,
                active_bridge_count,
                reported_at_ms,
            },
        )
    }
}
