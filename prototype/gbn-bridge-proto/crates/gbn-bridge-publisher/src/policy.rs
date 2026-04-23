use gbn_bridge_protocol::ReachabilityClass;

use crate::bridge_scoring;
use crate::registry;
use crate::storage::{BridgeRecord, InMemoryAuthorityStorage};
use crate::AuthorityPolicy;

pub fn is_creator_ingress_capable(reachability_class: &ReachabilityClass) -> bool {
    matches!(reachability_class, ReachabilityClass::Direct)
}

pub fn bootstrap_candidates(
    storage: &InMemoryAuthorityStorage,
    now_ms: u64,
    policy: &AuthorityPolicy,
) -> Vec<BridgeRecord> {
    let mut records = registry::active_bridge_records(storage, now_ms, false);
    if policy.direct_only_bootstrap {
        records.retain(|record| is_creator_ingress_capable(&record.reachability_class));
    }
    bridge_scoring::sort_bootstrap_candidates(&mut records);
    records
}

pub fn catalog_candidates(
    storage: &InMemoryAuthorityStorage,
    now_ms: u64,
    policy: &AuthorityPolicy,
    request_direct_only: bool,
) -> Vec<BridgeRecord> {
    let mut records = registry::active_bridge_records(storage, now_ms, false);
    if request_direct_only || !policy.allow_non_direct_catalog_entries {
        records.retain(|record| is_creator_ingress_capable(&record.reachability_class));
    }
    bridge_scoring::sort_catalog_candidates(&mut records);
    records
}
