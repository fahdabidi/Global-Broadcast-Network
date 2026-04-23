use gbn_bridge_protocol::CreatorJoinRequest;
use gbn_bridge_publisher::AuthorityBootstrapPlan;

use crate::{CreatorRuntime, ExitBridgeRuntime, RuntimeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCreator {
    host_creator_id: String,
}

impl HostCreator {
    pub fn new(host_creator_id: impl Into<String>) -> Self {
        Self {
            host_creator_id: host_creator_id.into(),
        }
    }

    pub fn host_creator_id(&self) -> &str {
        &self.host_creator_id
    }

    pub fn forward_join_request(
        &self,
        creator: &CreatorRuntime,
        relay_bridge: &mut ExitBridgeRuntime,
        request_id: &str,
        now_ms: u64,
    ) -> RuntimeResult<AuthorityBootstrapPlan> {
        let request = CreatorJoinRequest {
            request_id: request_id.to_string(),
            host_creator_id: self.host_creator_id.clone(),
            relay_bridge_id: relay_bridge.config().bridge_id.clone(),
            creator: creator.pending_creator(),
        };

        relay_bridge
            .publisher_client_mut()
            .begin_bootstrap(request, now_ms)
            .map_err(Into::into)
    }
}
