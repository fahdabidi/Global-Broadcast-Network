use gbn_bridge_protocol::{BridgeLease, ReachabilityClass};

#[derive(Debug, Clone, Default)]
pub struct LeaseState {
    current_lease: Option<BridgeLease>,
    last_registration_at_ms: Option<u64>,
    last_heartbeat_sent_at_ms: Option<u64>,
}

impl LeaseState {
    pub fn current(&self) -> Option<&BridgeLease> {
        self.current_lease.as_ref()
    }

    pub fn current_cloned(&self) -> Option<BridgeLease> {
        self.current_lease.clone()
    }

    pub fn update_lease(&mut self, lease: BridgeLease, registered_at_ms: u64) {
        self.current_lease = Some(lease);
        self.last_registration_at_ms = Some(registered_at_ms);
    }

    pub fn clear(&mut self) {
        self.current_lease = None;
    }

    pub fn mark_heartbeat_sent(&mut self, now_ms: u64) {
        self.last_heartbeat_sent_at_ms = Some(now_ms);
    }

    pub fn last_heartbeat_sent_at_ms(&self) -> Option<u64> {
        self.last_heartbeat_sent_at_ms
    }

    pub fn last_registration_at_ms(&self) -> Option<u64> {
        self.last_registration_at_ms
    }

    pub fn reachability_class(&self) -> Option<ReachabilityClass> {
        self.current_lease
            .as_ref()
            .map(|lease| lease.reachability_class.clone())
    }

    pub fn is_valid(&self, now_ms: u64) -> bool {
        self.current_lease
            .as_ref()
            .is_some_and(|lease| lease.lease_expiry_ms >= now_ms)
    }

    pub fn ingress_allowed(&self, now_ms: u64) -> bool {
        self.current_lease.as_ref().is_some_and(|lease| {
            lease.lease_expiry_ms >= now_ms
                && matches!(lease.reachability_class, ReachabilityClass::Direct)
        })
    }

    pub fn assigned_udp_punch_port(&self) -> Option<u16> {
        self.current_lease
            .as_ref()
            .map(|lease| lease.udp_punch_port)
    }
}
