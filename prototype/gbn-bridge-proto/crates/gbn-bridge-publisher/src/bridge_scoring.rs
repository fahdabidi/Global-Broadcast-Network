use std::cmp::Reverse;

use gbn_bridge_protocol::ReachabilityClass;

use crate::storage::BridgeRecord;

fn class_rank(class: &ReachabilityClass) -> u8 {
    match class {
        ReachabilityClass::Direct => 0,
        ReachabilityClass::Brokered => 1,
        ReachabilityClass::RelayOnly => 2,
    }
}

pub fn sort_catalog_candidates(records: &mut [BridgeRecord]) {
    records.sort_by(|left, right| {
        class_rank(&left.reachability_class)
            .cmp(&class_rank(&right.reachability_class))
            .then_with(|| {
                Reverse(left.current_lease.lease_expiry_ms)
                    .cmp(&Reverse(right.current_lease.lease_expiry_ms))
            })
            .then_with(|| left.bridge_id.cmp(&right.bridge_id))
    });
}

pub fn sort_bootstrap_candidates(records: &mut [BridgeRecord]) {
    records.sort_by(|left, right| {
        Reverse(left.current_lease.lease_expiry_ms)
            .cmp(&Reverse(right.current_lease.lease_expiry_ms))
            .then_with(|| left.bridge_id.cmp(&right.bridge_id))
    });
}
