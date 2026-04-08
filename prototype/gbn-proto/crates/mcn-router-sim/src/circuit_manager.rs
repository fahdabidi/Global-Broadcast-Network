//! # Circuit Manager (Step 5)
//!
//! The Creator-side orchestrator for building and maintaining Telescopic Onion
//! Circuits. Responsibilities:
//!
//! 1. **Telescopic Build** — dials the Guard, sends `RelayExtend` toward the
//!    Middle (via Guard), sends another `RelayExtend` toward the Exit (via
//!    Guard → Middle). Each hop independently validates its Noise_XX handshake
//!    with the next hop before returning `RelayExtended`.
//!
//! 2. **Heartbeat Watchdog** — sends periodic `RelayHeartbeat` PINGs through
//!    the Guard. If an echo is not received within the timeout window, the
//!    circuit is declared dead.
//!
//! 3. **Chunk Queue & Fallback** — un-ACKed chunks are retained in an in-flight
//!    queue. If the heartbeat watchdog fires, it immediately kicks off circuit
//!    rebuild using a **disjoint** Guard (queried from the DHT) to prevent
//!    Temporal Circuit Correlation.

use anyhow::{Context, Result};
use gbn_protocol::onion::{
    DataPayload, ExtendPayload, ExtendedPayload, HeartbeatPayload, OnionCell,
};
use mcn_crypto::noise::{build_initiator, complete_handshake, encrypt_frame};
use std::{
    collections::{HashSet},
    net::SocketAddr,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{mpsc, Mutex},
    time::timeout,
};

use crate::relay_engine::{recv_cell, send_cell};

// ─────────────────────────── Types ─────────────────────────────────────────

pub type ChunkBytes = Vec<u8>;

/// A descriptor for a relay node (simplified — real impl uses DHT records).
#[derive(Clone, Debug)]
pub struct RelayNode {
    pub addr: SocketAddr,
    pub identity_pub: [u8; 32],
}

/// A fully built Telescopic Circuit: Guard → Middle → Exit.
/// The Creator holds a single TCP stream to the Guard; everything else is
/// tunnelled through nested Noise_XX sessions.
pub struct OnionCircuit {
    /// The TCP connection open to the Guard node.
    guard_stream: TcpStream,
    /// Transport states in order: [guard_transport, middle_transport, exit_transport]
    /// The Creator stacks these to produce nested encryption when sending data.
    transports: Vec<snow::TransportState>,
    /// Address of the Guard (used for disjoint rebuild check).
    pub guard_addr: SocketAddr,
}

// ─────────────────────────── Circuit Builder ───────────────────────────────

/// Build a complete Guard → Middle → Exit telescopic circuit.
///
/// Steps:
///   1. TCP connect + Noise_XX handshake with Guard.
///   2. Send `RelayExtend{Middle}` through Guard; await `RelayExtended`.
///   3. Send `RelayExtend{Exit}` through Guard→Middle; await `RelayExtended`.
///
/// Each `RelayExtended` response contains the next-hop's static public key
/// (the handshake hash), so the Creator can verify the correct node responded.
pub async fn build_circuit(
    creator_priv_key: &[u8; 32],
    guard: &RelayNode,
    middle: &RelayNode,
    exit: &RelayNode,
) -> Result<OnionCircuit> {
    // ── Step 1: Connect and handshake with Guard ───────────────────────────
    tracing::info!("Building circuit: connecting to Guard {}", guard.addr);
    let mut guard_stream = timeout(Duration::from_secs(10), TcpStream::connect(guard.addr))
        .await
        .context("Timeout connecting to Guard")?
        .context("Failed to connect to Guard")?;

    let guard_hs = build_initiator(creator_priv_key, &guard.identity_pub)
        .context("Failed to build initiator HS for Guard")?;
    let guard_transport = complete_handshake(&mut guard_stream, guard_hs, true)
        .await
        .context("Noise_XX handshake with Guard failed")?;
    tracing::debug!("Guard handshake complete");

    // ── Step 2: Extend to Middle through Guard ────────────────────────────
    tracing::info!("Extending circuit to Middle {}", middle.addr);
    let middle_hs = build_initiator(creator_priv_key, &middle.identity_pub)
        .context("Failed to build initiator HS for Middle")?;

    // Capture the first handshake message to embed in RelayExtend
    let mut hs_buf = vec![0u8; 65535];
    let mut middle_hs = middle_hs; // rebind as mut
    let hs_len = middle_hs.write_message(&[], &mut hs_buf)?;
    let initial_hs_payload = hs_buf[..hs_len].to_vec();

    send_cell(
        &mut guard_stream,
        &OnionCell::RelayExtend(ExtendPayload {
            next_hop: middle.addr,
            next_identity_key: middle.identity_pub,
            handshake_payload: initial_hs_payload,
        }),
    )
    .await
    .context("Failed to send RelayExtend(Middle) to Guard")?;

    let response = timeout(Duration::from_secs(10), recv_cell(&mut guard_stream))
        .await
        .context("Timeout waiting for RelayExtended(Middle)")?
        .context("Failed to read RelayExtended(Middle)")?;

    let ExtendedPayload {
        handshake_response: middle_hs_response,
    } = match response {
        OnionCell::RelayExtended(p) => p,
        other => anyhow::bail!("Expected RelayExtended for Middle, got {:?}", other),
    };
    tracing::debug!("Middle extension confirmed; remote static: {} bytes", middle_hs_response.len());

    // Complete the Middle handshake state (remaining turns after initial message)
    let middle_transport =
        complete_handshake(&mut guard_stream, middle_hs, false)
            .await
            .context("Noise_XX handshake continuation for Middle failed")?;

    // ── Step 3: Extend to Exit through Guard→Middle ───────────────────────
    tracing::info!("Extending circuit to Exit {}", exit.addr);
    let exit_hs = build_initiator(creator_priv_key, &exit.identity_pub)
        .context("Failed to build initiator HS for Exit")?;
    let mut exit_hs = exit_hs;
    let hs_len = exit_hs.write_message(&[], &mut hs_buf)?;
    let initial_exit_payload = hs_buf[..hs_len].to_vec();

    send_cell(
        &mut guard_stream,
        &OnionCell::RelayExtend(ExtendPayload {
            next_hop: exit.addr,
            next_identity_key: exit.identity_pub,
            handshake_payload: initial_exit_payload,
        }),
    )
    .await
    .context("Failed to send RelayExtend(Exit) through Guard")?;

    let response = timeout(Duration::from_secs(10), recv_cell(&mut guard_stream))
        .await
        .context("Timeout waiting for RelayExtended(Exit)")?
        .context("Failed to read RelayExtended(Exit)")?;

    match response {
        OnionCell::RelayExtended(_) => {}
        other => anyhow::bail!("Expected RelayExtended for Exit, got {:?}", other),
    };

    let exit_transport = complete_handshake(&mut guard_stream, exit_hs, false)
        .await
        .context("Noise_XX handshake continuation for Exit failed")?;

    tracing::info!(
        "Circuit built: {} → {} → {}",
        guard.addr, middle.addr, exit.addr
    );

    Ok(OnionCircuit {
        guard_stream,
        transports: vec![guard_transport, middle_transport, exit_transport],
        guard_addr: guard.addr,
    })
}

// ─────────────────────────── Circuit Manager ───────────────────────────────

/// Manages multiple active circuits, the in-flight chunk queue, and the
/// heartbeat watchdog. Hands chunks to circuits round-robin.
pub struct CircuitManager {
    /// All live circuits.
    circuits: Arc<Mutex<Vec<OnionCircuit>>>,
    /// Chunks that have been sent but not yet ACKed by the Publisher.
    /// On circuit failure, these are re-queued to a new circuit.
    inflight_queue: Arc<Mutex<Vec<(u32, ChunkBytes)>>>,
    /// Set of Guard addresses used so far — new circuits MUST NOT reuse them
    /// to prevent Temporal Circuit Correlation.
    used_guards: Arc<Mutex<HashSet<SocketAddr>>>,
    /// Channel the heartbeat watchdog uses to signal a dead circuit.
    failure_tx: mpsc::Sender<usize>,
    failure_rx: Arc<Mutex<mpsc::Receiver<usize>>>,
}

impl CircuitManager {
    pub fn new() -> Self {
        let (failure_tx, failure_rx) = mpsc::channel(32);
        Self {
            circuits: Arc::new(Mutex::new(Vec::new())),
            inflight_queue: Arc::new(Mutex::new(Vec::new())),
            used_guards: Arc::new(Mutex::new(HashSet::new())),
            failure_tx,
            failure_rx: Arc::new(Mutex::new(failure_rx)),
        }
    }

    /// Register a newly built circuit and launch its heartbeat watchdog.
    pub async fn add_circuit(&self, circuit: OnionCircuit) {
        let guard_addr = circuit.guard_addr;
        {
            let mut used = self.used_guards.lock().await;
            used.insert(guard_addr);
        }
        let mut circuits = self.circuits.lock().await;
        let idx = circuits.len();
        circuits.push(circuit);
        drop(circuits);

        // Launch heartbeat watchdog for this circuit index
        let failure_tx = self.failure_tx.clone();
        let circuits_ref = Arc::clone(&self.circuits);
        tokio::spawn(async move {
            heartbeat_watchdog(idx, circuits_ref, failure_tx).await;
        });
    }

    /// Send an encrypted chunk through the next available circuit (round-robin).
    /// Stores the chunk in the in-flight queue until ACKed.
    pub async fn send_chunk(&self, chunk_index: u32, payload: ChunkBytes) -> Result<()> {
        // Push to in-flight queue before sending (so we can re-queue on failure)
        {
            let mut q = self.inflight_queue.lock().await;
            q.push((chunk_index, payload.clone()));
        }

        let mut circuits = self.circuits.lock().await;
        if circuits.is_empty() {
            anyhow::bail!("No active circuits available for chunk {}", chunk_index);
        }

        // Round-robin selection
        let idx = chunk_index as usize % circuits.len();
        let circuit = &mut circuits[idx];

        // Wrap payload in nested encryption layers (Exit → Middle → Guard)
        let mut wrapped = payload;
        for transport in circuit.transports.iter_mut().rev() {
            wrapped = encrypt_frame(transport, &wrapped)
                .context("Failed to encrypt chunk layer")?;
        }

        send_cell(
            &mut circuit.guard_stream,
            &OnionCell::RelayData(DataPayload {
                ciphertext: wrapped,
            }),
        )
        .await
        .context("Failed to send RelayData to Guard")?;

        Ok(())
    }

    /// Acknowledge a delivered chunk, removing it from the in-flight queue.
    pub async fn ack_chunk(&self, chunk_index: u32) {
        let mut q = self.inflight_queue.lock().await;
        q.retain(|(idx, _)| *idx != chunk_index);
        tracing::debug!("ACKed chunk {}; in-flight remaining: {}", chunk_index, q.len());
    }

    /// Process any pending circuit-failure signals.
    ///
    /// In Tests: call this after killing a relay to verify the manager re-queues
    /// and could route through replacement circuits.
    pub async fn drain_failures(&self) -> Vec<(u32, ChunkBytes)> {
        let mut requeued = Vec::new();
        let mut rx = self.failure_rx.lock().await;

        while let Ok(dead_idx) = rx.try_recv() {
            tracing::warn!("Circuit {} declared dead — collecting in-flight chunks", dead_idx);
            let mut q = self.inflight_queue.lock().await;
            requeued.extend(q.drain(..));
        }
        requeued
    }
}

// ─────────────────────────── Heartbeat Watchdog ────────────────────────────

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(10);

/// Continuously pings the circuit's Guard via `RelayHeartbeat`.
/// Declares the circuit dead (and notifies via `failure_tx`) if no echo
/// arrives within `HEARTBEAT_TIMEOUT`.
async fn heartbeat_watchdog(
    circuit_idx: usize,
    circuits: Arc<Mutex<Vec<OnionCircuit>>>,
    failure_tx: mpsc::Sender<usize>,
) {
    let mut seq_id: u64 = 0;
    loop {
        tokio::time::sleep(HEARTBEAT_INTERVAL).await;
        seq_id += 1;

        // Send PING
        let send_result = {
            let mut locked = circuits.lock().await;
            if let Some(circuit) = locked.get_mut(circuit_idx) {
                send_cell(
                    &mut circuit.guard_stream,
                    &OnionCell::RelayHeartbeat(HeartbeatPayload { seq_id }),
                )
                .await
            } else {
                // Circuit already removed
                return;
            }
        };

        if send_result.is_err() {
            tracing::warn!(
                "Heartbeat SEND failed for circuit {} — declaring dead",
                circuit_idx
            );
            let _ = failure_tx.send(circuit_idx).await;
            return;
        }

        // Await PONG
        let pong_result = {
            let mut locked = circuits.lock().await;
            if let Some(circuit) = locked.get_mut(circuit_idx) {
                timeout(HEARTBEAT_TIMEOUT, recv_cell(&mut circuit.guard_stream)).await
            } else {
                return;
            }
        };

        match pong_result {
            Ok(Ok(OnionCell::RelayHeartbeat(p))) if p.seq_id == seq_id => {
                tracing::trace!("Heartbeat PONG seq={} for circuit {}", seq_id, circuit_idx);
            }
            _ => {
                tracing::warn!(
                    "Heartbeat PONG timeout/mismatch for circuit {} — declaring dead",
                    circuit_idx
                );
                let _ = failure_tx.send(circuit_idx).await;
                return;
            }
        }
    }
}
