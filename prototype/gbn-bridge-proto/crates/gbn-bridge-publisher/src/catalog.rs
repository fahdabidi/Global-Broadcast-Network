use ed25519_dalek::SigningKey;
use gbn_bridge_protocol::{
    BridgeCatalogRequest, BridgeCatalogResponse, BridgeCatalogResponseUnsigned, BridgeDescriptor,
    BridgeDescriptorUnsigned,
};

use crate::registry;
use crate::storage::InMemoryAuthorityStorage;
use crate::{AuthorityConfig, AuthorityError, AuthorityPolicy, AuthorityResult};

pub fn issue_catalog(
    storage: &mut InMemoryAuthorityStorage,
    signing_key: &SigningKey,
    config: &AuthorityConfig,
    policy: &AuthorityPolicy,
    request: &BridgeCatalogRequest,
    now_ms: u64,
) -> AuthorityResult<BridgeCatalogResponse> {
    let direct_only = request.direct_only || !policy.allow_non_direct_catalog_entries;
    let bridges = registry::active_bridge_records(storage, now_ms, direct_only)
        .into_iter()
        .map(|record| {
            BridgeDescriptor::sign(
                BridgeDescriptorUnsigned {
                    bridge_id: record.bridge_id,
                    identity_pub: record.identity_pub,
                    ingress_endpoints: record.ingress_endpoints,
                    udp_punch_port: record.assigned_udp_punch_port,
                    reachability_class: record.reachability_class,
                    lease_expiry_ms: record.current_lease.lease_expiry_ms,
                    capabilities: record.capabilities,
                },
                signing_key,
            )
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(AuthorityError::from)?;

    let unsigned = BridgeCatalogResponseUnsigned {
        catalog_id: storage.next_catalog_id(),
        issued_at_ms: now_ms,
        expires_at_ms: now_ms + config.catalog_ttl_ms,
        bridges,
    };

    BridgeCatalogResponse::sign(unsigned, signing_key).map_err(Into::into)
}
