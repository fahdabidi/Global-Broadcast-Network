use std::collections::BTreeMap;

use gbn_bridge_protocol::{
    BootstrapDhtEntry, BridgeCapability, BridgeHeartbeat, BridgeIngressEndpoint, BridgeLease,
    CreatorJoinRequest, PublicKeyBytes, ReachabilityClass, RevocationReason, UnixTimestampMs,
};

#[derive(Debug, Clone)]
pub struct BridgeRecord {
    pub bridge_id: String,
    pub identity_pub: PublicKeyBytes,
    pub ingress_endpoints: Vec<BridgeIngressEndpoint>,
    pub assigned_udp_punch_port: u16,
    pub reachability_class: ReachabilityClass,
    pub capabilities: Vec<BridgeCapability>,
    pub current_lease: BridgeLease,
    pub last_heartbeat: BridgeHeartbeat,
    pub revoked_reason: Option<RevocationReason>,
    pub revoked_at_ms: Option<UnixTimestampMs>,
}

impl BridgeRecord {
    pub fn is_active(&self, now_ms: UnixTimestampMs) -> bool {
        self.revoked_reason.is_none() && self.current_lease.lease_expiry_ms >= now_ms
    }

    pub fn is_direct(&self) -> bool {
        self.reachability_class == ReachabilityClass::Direct
    }
}

#[derive(Debug, Clone)]
pub struct BootstrapSessionRecord {
    pub bootstrap_session_id: String,
    pub creator_entry: BootstrapDhtEntry,
    pub host_creator_id: String,
    pub relay_bridge_id: String,
    pub seed_bridge_id: String,
    pub bridge_ids: Vec<String>,
    pub created_at_ms: UnixTimestampMs,
    pub response_expiry_ms: UnixTimestampMs,
}

#[derive(Debug, Clone)]
pub struct PendingBatchAssignment {
    pub bootstrap_session_id: String,
    pub join_request: CreatorJoinRequest,
    pub creator_entry: BootstrapDhtEntry,
}

#[derive(Debug, Clone)]
pub struct BatchWindowState {
    pub batch_id: String,
    pub window_started_at_ms: UnixTimestampMs,
    pub assignments: Vec<PendingBatchAssignment>,
}

#[derive(Debug, Default)]
pub struct InMemoryAuthorityStorage {
    pub bridges: BTreeMap<String, BridgeRecord>,
    pub bootstrap_sessions: BTreeMap<String, BootstrapSessionRecord>,
    pub current_batch: Option<BatchWindowState>,
    next_lease_seq: u64,
    next_catalog_seq: u64,
    next_bootstrap_seq: u64,
    next_batch_seq: u64,
}

impl InMemoryAuthorityStorage {
    pub fn next_lease_id(&mut self) -> String {
        self.next_lease_seq += 1;
        format!("lease-{:06}", self.next_lease_seq)
    }

    pub fn next_catalog_id(&mut self) -> String {
        self.next_catalog_seq += 1;
        format!("catalog-{:06}", self.next_catalog_seq)
    }

    pub fn next_bootstrap_id(&mut self) -> String {
        self.next_bootstrap_seq += 1;
        format!("bootstrap-{:06}", self.next_bootstrap_seq)
    }

    pub fn next_batch_id(&mut self) -> String {
        self.next_batch_seq += 1;
        format!("batch-{:06}", self.next_batch_seq)
    }
}
