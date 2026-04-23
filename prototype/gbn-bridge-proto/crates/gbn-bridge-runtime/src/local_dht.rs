use std::collections::BTreeMap;

use gbn_bridge_protocol::{BootstrapDhtEntry, BridgeDescriptor, PublicKeyBytes, ReachabilityClass};

use crate::RuntimeResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalHintSource {
    Catalog,
    Bootstrap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalDhtNode {
    pub node_id: String,
    pub ip_addr: String,
    pub pub_key: PublicKeyBytes,
    pub udp_punch_port: u16,
    pub expires_at_ms: u64,
    pub reachability_class: ReachabilityClass,
    pub source: LocalHintSource,
    pub last_updated_ms: u64,
    pub active_tunnel_since_ms: Option<u64>,
}

#[derive(Debug, Clone, Default)]
pub struct LocalDht {
    nodes: BTreeMap<String, LocalDhtNode>,
}

impl LocalDht {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn node(&self, node_id: &str) -> Option<&LocalDhtNode> {
        self.nodes.get(node_id)
    }

    pub fn snapshot(&self) -> Vec<LocalDhtNode> {
        self.nodes.values().cloned().collect()
    }

    pub fn upsert_catalog_bridges(
        &mut self,
        bridges: &[BridgeDescriptor],
        publisher_key: &PublicKeyBytes,
        now_ms: u64,
    ) -> RuntimeResult<usize> {
        for bridge in bridges {
            bridge.verify_authority(publisher_key, now_ms)?;
            if let Some(endpoint) = bridge.ingress_endpoints.first() {
                let active_tunnel_since_ms = self
                    .nodes
                    .get(&bridge.bridge_id)
                    .and_then(|existing| existing.active_tunnel_since_ms);
                self.nodes.insert(
                    bridge.bridge_id.clone(),
                    LocalDhtNode {
                        node_id: bridge.bridge_id.clone(),
                        ip_addr: endpoint.host.clone(),
                        pub_key: bridge.identity_pub.clone(),
                        udp_punch_port: bridge.udp_punch_port,
                        expires_at_ms: bridge.lease_expiry_ms,
                        reachability_class: bridge.reachability_class.clone(),
                        source: LocalHintSource::Catalog,
                        last_updated_ms: now_ms,
                        active_tunnel_since_ms,
                    },
                );
            }
        }

        Ok(bridges.len())
    }

    pub fn upsert_bootstrap_entries(
        &mut self,
        entries: &[BootstrapDhtEntry],
        publisher_key: &PublicKeyBytes,
        now_ms: u64,
    ) -> RuntimeResult<usize> {
        for entry in entries {
            entry.verify_authority(publisher_key, now_ms)?;
            let active_tunnel_since_ms = self
                .nodes
                .get(&entry.node_id)
                .and_then(|existing| existing.active_tunnel_since_ms);
            self.nodes.insert(
                entry.node_id.clone(),
                LocalDhtNode {
                    node_id: entry.node_id.clone(),
                    ip_addr: entry.ip_addr.clone(),
                    pub_key: entry.pub_key.clone(),
                    udp_punch_port: entry.udp_punch_port,
                    expires_at_ms: entry.entry_expiry_ms,
                    reachability_class: ReachabilityClass::Direct,
                    source: LocalHintSource::Bootstrap,
                    last_updated_ms: now_ms,
                    active_tunnel_since_ms,
                },
            );
        }

        Ok(entries.len())
    }

    pub fn mark_tunnel_active(&mut self, node_id: &str, established_at_ms: u64) -> bool {
        let Some(node) = self.nodes.get_mut(node_id) else {
            return false;
        };

        node.active_tunnel_since_ms = Some(established_at_ms);
        true
    }
}
