use std::collections::BTreeSet;

use gbn_bridge_protocol::{
    BridgeCatalogResponse, BridgeDescriptor, PublicKeyBytes, ReachabilityClass,
};

use crate::{RuntimeError, RuntimeResult};

pub fn ordered_direct_bridges(
    catalog: &BridgeCatalogResponse,
    publisher_key: &PublicKeyBytes,
    now_ms: u64,
    excluded_bridge_ids: &BTreeSet<String>,
) -> RuntimeResult<Vec<BridgeDescriptor>> {
    catalog.verify_authority(publisher_key, now_ms)?;

    let mut bridges: Vec<_> = catalog
        .bridges
        .iter()
        .filter(|bridge| matches!(bridge.reachability_class, ReachabilityClass::Direct))
        .filter(|bridge| !excluded_bridge_ids.contains(&bridge.bridge_id))
        .cloned()
        .collect();

    bridges.sort_by(|left, right| {
        right
            .lease_expiry_ms
            .cmp(&left.lease_expiry_ms)
            .then_with(|| left.bridge_id.cmp(&right.bridge_id))
    });

    if bridges.is_empty() {
        return Err(RuntimeError::NoUsableBridgeCandidate);
    }

    Ok(bridges)
}

pub fn select_next_direct_bridge(
    catalog: &BridgeCatalogResponse,
    publisher_key: &PublicKeyBytes,
    now_ms: u64,
    excluded_bridge_ids: &BTreeSet<String>,
) -> RuntimeResult<BridgeDescriptor> {
    ordered_direct_bridges(catalog, publisher_key, now_ms, excluded_bridge_ids)?
        .into_iter()
        .next()
        .ok_or(RuntimeError::NoUsableBridgeCandidate)
}
