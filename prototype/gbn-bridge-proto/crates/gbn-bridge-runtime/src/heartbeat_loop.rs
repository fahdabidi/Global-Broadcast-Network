use gbn_bridge_protocol::BridgeHeartbeat;

use crate::lease_state::LeaseState;

#[derive(Debug, Clone, Default)]
pub struct HeartbeatLoop;

impl HeartbeatLoop {
    pub fn maybe_build_heartbeat(
        &self,
        lease_state: &LeaseState,
        bridge_id: &str,
        active_sessions: u32,
        now_ms: u64,
    ) -> Option<BridgeHeartbeat> {
        let lease = lease_state.current()?;
        if let Some(last_sent) = lease_state.last_heartbeat_sent_at_ms() {
            if now_ms.saturating_sub(last_sent) < lease.heartbeat_interval_ms {
                return None;
            }
        }

        Some(BridgeHeartbeat {
            lease_id: lease.lease_id.clone(),
            bridge_id: bridge_id.to_string(),
            heartbeat_at_ms: now_ms,
            active_sessions,
            observed_ingress: None,
        })
    }
}
