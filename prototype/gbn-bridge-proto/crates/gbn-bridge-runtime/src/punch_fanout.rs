use std::collections::{BTreeMap, BTreeSet};

use gbn_bridge_protocol::{BootstrapDhtEntry, BridgeCatalogResponse, PublicKeyBytes};

use crate::selector;
use crate::{RuntimeError, RuntimeResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanoutSource {
    CatalogRefresh,
    Bootstrap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatorPunchAttempt {
    pub bootstrap_session_id: String,
    pub target_node_id: String,
    pub target_ip_addr: String,
    pub target_pub_key: PublicKeyBytes,
    pub target_udp_punch_port: u16,
    pub attempt_expiry_ms: u64,
    pub probe_nonce: u64,
    pub started_at_ms: u64,
    pub source: FanoutSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatorPunchAck {
    pub bootstrap_session_id: String,
    pub target_node_id: String,
    pub acked_probe_nonce: u64,
    pub established_at_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PunchFanout {
    attempts: BTreeMap<String, CreatorPunchAttempt>,
    next_refresh_session_seq: u64,
    next_probe_nonce: u64,
}

impl PunchFanout {
    pub fn active_attempts(&self) -> Vec<CreatorPunchAttempt> {
        self.attempts.values().cloned().collect()
    }

    pub fn begin_for_catalog(
        &mut self,
        catalog: &BridgeCatalogResponse,
        publisher_key: &PublicKeyBytes,
        now_ms: u64,
    ) -> RuntimeResult<Vec<CreatorPunchAttempt>> {
        self.next_refresh_session_seq += 1;
        let refresh_prefix = format!("refresh-{:06}", self.next_refresh_session_seq);
        let direct_bridges =
            selector::ordered_direct_bridges(catalog, publisher_key, now_ms, &BTreeSet::new())?;

        Ok(direct_bridges
            .into_iter()
            .map(|bridge| {
                self.insert_attempt(
                    format!("{refresh_prefix}::{}", bridge.bridge_id),
                    bridge.bridge_id,
                    bridge
                        .ingress_endpoints
                        .first()
                        .map(|endpoint| endpoint.host.clone())
                        .unwrap_or_default(),
                    bridge.identity_pub,
                    bridge.udp_punch_port,
                    bridge.lease_expiry_ms,
                    now_ms,
                    FanoutSource::CatalogRefresh,
                )
            })
            .collect())
    }

    pub fn begin_for_bootstrap_entries(
        &mut self,
        bootstrap_session_id: &str,
        entries: &[BootstrapDhtEntry],
        publisher_key: &PublicKeyBytes,
        now_ms: u64,
    ) -> RuntimeResult<Vec<CreatorPunchAttempt>> {
        let mut attempts = Vec::with_capacity(entries.len());
        for entry in entries {
            entry.verify_authority(publisher_key, now_ms)?;
            attempts.push(self.insert_attempt(
                format!("{bootstrap_session_id}::{}", entry.node_id),
                entry.node_id.clone(),
                entry.ip_addr.clone(),
                entry.pub_key.clone(),
                entry.udp_punch_port,
                entry.entry_expiry_ms,
                now_ms,
                FanoutSource::Bootstrap,
            ));
        }

        Ok(attempts)
    }

    pub fn acknowledge(
        &mut self,
        bootstrap_session_id: &str,
        target_node_id: &str,
        acked_probe_nonce: u64,
        established_at_ms: u64,
    ) -> RuntimeResult<CreatorPunchAck> {
        let attempt = self.attempts.get(bootstrap_session_id).ok_or_else(|| {
            RuntimeError::CreatorBootstrapSessionNotTracked {
                bootstrap_session_id: bootstrap_session_id.to_string(),
            }
        })?;

        if attempt.target_node_id != target_node_id {
            return Err(RuntimeError::CreatorPunchTargetMismatch {
                bootstrap_session_id: bootstrap_session_id.to_string(),
                expected_target_id: attempt.target_node_id.clone(),
                actual_target_id: target_node_id.to_string(),
            });
        }

        if established_at_ms > attempt.attempt_expiry_ms {
            return Err(RuntimeError::PunchAttemptExpired {
                bootstrap_session_id: bootstrap_session_id.to_string(),
                attempt_expiry_ms: attempt.attempt_expiry_ms,
                now_ms: established_at_ms,
            });
        }

        if acked_probe_nonce != attempt.probe_nonce {
            return Err(RuntimeError::ProbeNonceMismatch {
                bootstrap_session_id: bootstrap_session_id.to_string(),
                expected: attempt.probe_nonce,
                actual: acked_probe_nonce,
            });
        }

        Ok(CreatorPunchAck {
            bootstrap_session_id: bootstrap_session_id.to_string(),
            target_node_id: target_node_id.to_string(),
            acked_probe_nonce,
            established_at_ms,
        })
    }

    fn insert_attempt(
        &mut self,
        bootstrap_session_id: String,
        target_node_id: String,
        target_ip_addr: String,
        target_pub_key: PublicKeyBytes,
        target_udp_punch_port: u16,
        attempt_expiry_ms: u64,
        started_at_ms: u64,
        source: FanoutSource,
    ) -> CreatorPunchAttempt {
        self.next_probe_nonce += 1;
        let attempt = CreatorPunchAttempt {
            bootstrap_session_id: bootstrap_session_id.clone(),
            target_node_id,
            target_ip_addr,
            target_pub_key,
            target_udp_punch_port,
            attempt_expiry_ms,
            probe_nonce: self.next_probe_nonce,
            started_at_ms,
            source,
        };

        self.attempts.insert(bootstrap_session_id, attempt.clone());

        attempt
    }
}
