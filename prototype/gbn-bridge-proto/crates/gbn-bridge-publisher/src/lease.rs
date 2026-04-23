use ed25519_dalek::SigningKey;
use gbn_bridge_protocol::{
    BridgeHeartbeat, BridgeLease, BridgeRegister, ReachabilityClass, RevocationReason,
};

use crate::registry;
use crate::storage::InMemoryAuthorityStorage;
use crate::{AuthorityConfig, AuthorityResult};

pub fn register_bridge(
    storage: &mut InMemoryAuthorityStorage,
    signing_key: &SigningKey,
    config: &AuthorityConfig,
    request: BridgeRegister,
    reachability_class: ReachabilityClass,
    now_ms: u64,
) -> AuthorityResult<BridgeLease> {
    registry::register_bridge(
        storage,
        signing_key,
        config,
        request,
        reachability_class,
        now_ms,
    )
}

pub fn handle_heartbeat(
    storage: &mut InMemoryAuthorityStorage,
    signing_key: &SigningKey,
    config: &AuthorityConfig,
    heartbeat: BridgeHeartbeat,
) -> AuthorityResult<BridgeLease> {
    registry::renew_lease_from_heartbeat(storage, signing_key, config, heartbeat)
}

pub fn reclassify_bridge(
    storage: &mut InMemoryAuthorityStorage,
    signing_key: &SigningKey,
    config: &AuthorityConfig,
    bridge_id: &str,
    reachability_class: ReachabilityClass,
    udp_punch_port: Option<u16>,
    now_ms: u64,
) -> AuthorityResult<BridgeLease> {
    registry::reclassify_bridge(
        storage,
        signing_key,
        config,
        bridge_id,
        reachability_class,
        udp_punch_port,
        now_ms,
    )
}

pub fn revoke_bridge(
    storage: &mut InMemoryAuthorityStorage,
    signing_key: &SigningKey,
    bridge_id: &str,
    reason: RevocationReason,
    now_ms: u64,
) -> AuthorityResult<gbn_bridge_protocol::BridgeRevoke> {
    registry::revoke_bridge(storage, signing_key, bridge_id, reason, now_ms)
}
