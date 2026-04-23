use gbn_bridge_protocol::{BridgePunchAck, BridgePunchProbe, BridgeSetRequest, BridgeSetResponse};
use gbn_bridge_publisher::AuthorityBootstrapPlan;

use crate::{CreatorRuntime, ExitBridgeRuntime, HostCreator, RuntimeError, RuntimeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedTunnelOutcome {
    pub probe: BridgePunchProbe,
    pub bridge_ack: BridgePunchAck,
}

pub fn request_first_contact(
    creator: &mut CreatorRuntime,
    host_creator: &HostCreator,
    relay_bridge: &mut ExitBridgeRuntime,
    request_id: &str,
    now_ms: u64,
) -> RuntimeResult<AuthorityBootstrapPlan> {
    let plan = host_creator.forward_join_request(creator, relay_bridge, request_id, now_ms)?;
    creator.apply_bootstrap_response(&plan.response, now_ms)?;
    creator.remember_self_entry(plan.creator_entry.clone(), now_ms)?;
    Ok(plan)
}

pub fn establish_seed_tunnel(
    creator: &mut CreatorRuntime,
    seed_bridge: &mut ExitBridgeRuntime,
    plan: &AuthorityBootstrapPlan,
    now_ms: u64,
) -> RuntimeResult<SeedTunnelOutcome> {
    if seed_bridge.config().bridge_id != plan.response.seed_bridge.node_id {
        return Err(RuntimeError::UnexpectedBridgeRuntime {
            expected_bridge_id: plan.response.seed_bridge.node_id.clone(),
            actual_bridge_id: seed_bridge.config().bridge_id.clone(),
        });
    }

    seed_bridge.apply_seed_assignment(plan, now_ms)?;
    let probe = seed_bridge.begin_publisher_directed_punch(plan.seed_punch.clone(), now_ms)?;
    let bridge_ack = seed_bridge.acknowledge_tunnel(
        &probe.bootstrap_session_id,
        &creator.config().creator_id,
        creator.config().udp_punch_port,
        probe.probe_nonce,
        now_ms,
    )?;
    creator.mark_bridge_active(&plan.response.seed_bridge.node_id, now_ms);

    Ok(SeedTunnelOutcome { probe, bridge_ack })
}

pub fn fetch_bridge_set(
    creator: &mut CreatorRuntime,
    seed_bridge: &mut ExitBridgeRuntime,
    plan: &AuthorityBootstrapPlan,
    now_ms: u64,
) -> RuntimeResult<BridgeSetResponse> {
    if seed_bridge.config().bridge_id != plan.response.seed_bridge.node_id {
        return Err(RuntimeError::UnexpectedBridgeRuntime {
            expected_bridge_id: plan.response.seed_bridge.node_id.clone(),
            actual_bridge_id: seed_bridge.config().bridge_id.clone(),
        });
    }

    let response = seed_bridge.serve_bridge_set(
        &BridgeSetRequest {
            bootstrap_session_id: plan.response.bootstrap_session_id.clone(),
            creator_id: creator.config().creator_id.clone(),
            requested_bridge_count: plan.response.assigned_bridge_count,
        },
        now_ms,
    )?;

    if response.bootstrap_session_id != plan.response.bootstrap_session_id {
        return Err(RuntimeError::BridgeSetSessionMismatch {
            expected_bootstrap_session_id: plan.response.bootstrap_session_id.clone(),
            actual_bootstrap_session_id: response.bootstrap_session_id,
        });
    }

    creator.store_bridge_set(&response, now_ms)?;
    Ok(response)
}
