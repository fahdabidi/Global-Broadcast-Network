use crate::{CreatorRuntime, RuntimeError, RuntimeResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgePoolEntry {
    pub bridge_id: String,
    pub activated_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BridgePool {
    entries: Vec<BridgePoolEntry>,
    target_bridge_count: usize,
}

impl BridgePool {
    pub fn from_creator(
        creator: &CreatorRuntime,
        target_bridge_count: usize,
    ) -> RuntimeResult<Self> {
        let mut entries: Vec<_> = creator
            .local_dht()
            .snapshot()
            .into_iter()
            .filter_map(|node| {
                node.active_tunnel_since_ms
                    .map(|activated_at_ms| BridgePoolEntry {
                        bridge_id: node.node_id,
                        activated_at_ms,
                    })
            })
            .collect();

        entries.sort_by(|left, right| {
            left.activated_at_ms
                .cmp(&right.activated_at_ms)
                .then_with(|| left.bridge_id.cmp(&right.bridge_id))
        });

        if entries.is_empty() {
            return Err(RuntimeError::NoActiveUploadBridge);
        }

        Ok(Self {
            entries,
            target_bridge_count,
        })
    }

    pub fn entries(&self) -> &[BridgePoolEntry] {
        &self.entries
    }

    pub fn target_bridge_count(&self) -> usize {
        self.target_bridge_count
    }

    pub fn active_bridge_count(&self) -> usize {
        self.entries.len()
    }

    pub fn selected_bridge_ids(&self) -> Vec<String> {
        self.entries
            .iter()
            .take(self.target_bridge_count.min(self.entries.len()))
            .map(|entry| entry.bridge_id.clone())
            .collect()
    }

    pub fn remove_bridge(&mut self, bridge_id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|entry| entry.bridge_id != bridge_id);
        before != self.entries.len()
    }

    pub fn contains(&self, bridge_id: &str) -> bool {
        self.entries
            .iter()
            .any(|entry| entry.bridge_id == bridge_id)
    }
}
