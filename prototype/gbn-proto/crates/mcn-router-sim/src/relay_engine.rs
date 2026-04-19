//! # Onion Relay Engine
//!
//! Relay behavior is decrypt-and-forward:
//! - Read one framed ciphertext from upstream.
//! - Open one onion layer with local static key (Noise_N).
//! - If `next_hop` exists, forward inner bytes to that hop and relay ACK back.
//! - If `next_hop` is `None`, reject the frame; the Publisher is the terminal
//!   recipient and the origin of the delivery ACK.

use crate::control::push_packet_meta_trace;
use anyhow::{Context, Result};
use gbn_protocol::onion::OnionLayer;
use mcn_crypto::noise::open;
use std::{net::SocketAddr, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::JoinHandle,
    time::timeout,
};

pub struct OnionRelayHandle {
    pub listen_addr: SocketAddr,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: JoinHandle<()>,
}

impl OnionRelayHandle {
    pub async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let _ = self.task.await;
    }
}

/// Write one raw frame: `[u32_be_len][bytes]`.
pub async fn write_raw_frame(stream: &mut TcpStream, data: &[u8]) -> Result<()> {
    let len = data.len() as u32;
    stream.write_all(&len.to_be_bytes()).await?;
    stream.write_all(data).await?;
    stream.flush().await?;
    Ok(())
}

/// Read one raw frame: `[u32_be_len][bytes]`.
pub async fn read_raw_frame(stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Spawn the onion relay listener.
pub async fn spawn_onion_relay(
    listen_addr: SocketAddr,
    local_priv_key: [u8; 32],
) -> Result<OnionRelayHandle> {
    let listener = TcpListener::bind(listen_addr).await?;
    let bound_addr = listener.local_addr()?;

    tracing::info!("OnionRelay listening on {}", bound_addr);

    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut shutdown_rx => {
                    tracing::info!("OnionRelay {} shutting down", bound_addr);
                    break;
                }
                accept_res = listener.accept() => {
                    match accept_res {
                        Ok((stream, peer_addr)) => {
                            let key = local_priv_key;
                            tokio::spawn(async move {
                                if let Err(e) = handle_onion_connection(stream, key).await {
                                    let err_chain = next_chain("");
                                    let msg = format!(
                                        "relay.handle_connection ERROR node={} listen={} peer={} err={e:#}",
                                        crate::trace::node_id(), bound_addr, peer_addr
                                    );
                                    tracing::error!("{}", msg);
                                    push_packet_meta_trace(
                                        "ComponentError",
                                        0,
                                        &msg,
                                        &err_chain,
                                        "relay.error",
                                    );
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("OnionRelay {} accept error: {}", bound_addr, e);
                        }
                    }
                }
            }
        }
    });

    Ok(OnionRelayHandle {
        listen_addr: bound_addr,
        shutdown_tx: Some(shutdown_tx),
        task,
    })
}

async fn handle_onion_connection(mut upstream: TcpStream, local_priv_key: [u8; 32]) -> Result<()> {
    let root_chain = next_chain("");

    // Read compound frame: [u32_be routing_len][routing_sealed][payload_sealed]
    let compound_frame = timeout(Duration::from_secs(30), read_raw_frame(&mut upstream))
        .await
        .context("Timeout reading upstream compound frame")?
        .context("Failed reading upstream compound frame")
        .map_err(|e| {
            push_packet_meta_trace(
                "ComponentError",
                0,
                &format!("relay.read_upstream ERROR err={e:#}"),
                &root_chain,
                "relay.error",
            );
            e
        })?;

    if compound_frame.len() < 4 {
        let msg = format!(
            "relay.read_upstream ERROR node={} err=compound frame too short: {} bytes",
            crate::trace::node_id(),
            compound_frame.len()
        );
        push_packet_meta_trace("ComponentError", 0, &msg, &root_chain, "relay.error");
        anyhow::bail!(msg);
    }
    let routing_len = u32::from_be_bytes(compound_frame[..4].try_into().unwrap()) as usize;
    if compound_frame.len() < 4 + routing_len {
        let msg = format!(
            "relay.read_upstream ERROR node={} err=routing_len={} exceeds frame size={}",
            crate::trace::node_id(),
            routing_len,
            compound_frame.len()
        );
        push_packet_meta_trace("ComponentError", 0, &msg, &root_chain, "relay.error");
        anyhow::bail!(msg);
    }
    let routing_bytes = &compound_frame[4..4 + routing_len];
    let payload_bytes = compound_frame[4 + routing_len..].to_vec();

    let layer_plain = open(&local_priv_key, routing_bytes)
        .context("Failed to open routing layer with local key")
        .map_err(|e| {
            push_packet_meta_trace(
                "ComponentError",
                routing_bytes.len(),
                &format!("relay.open_layer ERROR err={e:#}"),
                &root_chain,
                "relay.error",
            );
            e
        })?;
    let layer: OnionLayer = serde_json::from_slice(&layer_plain)
        .context("Failed to decode routing OnionLayer")
        .map_err(|e| {
            push_packet_meta_trace(
                "ComponentError",
                layer_plain.len(),
                &format!("relay.decode_layer ERROR err={e:#}"),
                &root_chain,
                "relay.error",
            );
            e
        })?;

    let incoming_chain = layer.trace_id.clone().unwrap_or_else(|| root_chain.clone());
    let ingress_chain = next_chain(&incoming_chain);
    push_packet_meta_trace(
        "ComponentInput",
        payload_bytes.len(),
        &format!(
            "relay.layer INPUT node={} next_hop={:?} routing_bytes={} payload_bytes={}",
            crate::trace::node_id(),
            layer.next_hop,
            routing_bytes.len(),
            payload_bytes.len()
        ),
        &ingress_chain,
        "relay.input",
    );

    match layer.next_hop {
        Some(next_hop) => {
            let relay_forward_chain = next_chain(&ingress_chain);
            push_packet_meta_trace(
                "RelayData(Intermediate)",
                payload_bytes.len(),
                &format!(
                    "relay.forward node={} next_hop={} payload_bytes={} exit={}",
                    crate::trace::node_id(),
                    next_hop,
                    payload_bytes.len(),
                    layer.inner.is_empty()
                ),
                &relay_forward_chain,
                "relay.data",
            );

            let mut next_stream = timeout(Duration::from_secs(10), TcpStream::connect(next_hop))
                .await
                .context(format!("Timeout dialing next hop {}", next_hop))?
                .context(format!("Failed connecting to next hop {}", next_hop))
                .map_err(|e| {
                    push_packet_meta_trace(
                        "ComponentError",
                        payload_bytes.len(),
                        &format!(
                            "relay.connect_next ERROR node={} next_hop={} err={e:#}",
                            crate::trace::node_id(),
                            next_hop
                        ),
                        &relay_forward_chain,
                        "relay.error",
                    );
                    e
                })?;

            if layer.inner.is_empty() {
                // Exit node: inner routing is empty — forward just payload_sealed to publisher.
                write_raw_frame(&mut next_stream, &payload_bytes)
                    .await
                    .context("Failed writing payload bytes to publisher")
                    .map_err(|e| {
                        push_packet_meta_trace(
                            "ComponentError",
                            payload_bytes.len(),
                            &format!(
                                "relay.write_next ERROR node={} next_hop={} err={e:#}",
                                crate::trace::node_id(),
                                next_hop
                            ),
                            &relay_forward_chain,
                            "relay.error",
                        );
                        e
                    })?;
            } else {
                // Intermediate node (Guard/Middle): forward [u32_be inner_routing_len][inner_routing][payload].
                let inner_routing_len = layer.inner.len() as u32;
                let mut forward = Vec::with_capacity(4 + layer.inner.len() + payload_bytes.len());
                forward.extend_from_slice(&inner_routing_len.to_be_bytes());
                forward.extend_from_slice(&layer.inner);
                forward.extend_from_slice(&payload_bytes);
                write_raw_frame(&mut next_stream, &forward)
                    .await
                    .context("Failed writing compound frame to next hop")
                    .map_err(|e| {
                        push_packet_meta_trace(
                            "ComponentError",
                            forward.len(),
                            &format!(
                                "relay.write_next ERROR node={} next_hop={} err={e:#}",
                                crate::trace::node_id(),
                                next_hop
                            ),
                            &relay_forward_chain,
                            "relay.error",
                        );
                        e
                    })?;
            }

            let ack_from_downstream =
                timeout(Duration::from_secs(30), read_raw_frame(&mut next_stream))
                    .await
                    .context("Timeout waiting for downstream ACK frame")?
                    .context("Failed reading downstream ACK frame")
                    .map_err(|e| {
                        push_packet_meta_trace(
                            "ComponentError",
                            payload_bytes.len(),
                            &format!(
                                "relay.read_ack_downstream ERROR node={} next_hop={} err={e:#}",
                                crate::trace::node_id(),
                                next_hop
                            ),
                            &relay_forward_chain,
                            "relay.error",
                        );
                        e
                    })?;

            // Peel one ACK layer (reverse onion) before relaying upstream.
            let (ack_to_upstream, ack_chain) =
                peel_ack_for_upstream(&local_priv_key, &ack_from_downstream)
                    .unwrap_or((ack_from_downstream, None));
            let upstream_chain = ack_chain.unwrap_or_else(|| next_chain(&relay_forward_chain));

            push_packet_meta_trace(
                "RelayAckPeel",
                ack_to_upstream.len(),
                &format!(
                    "relay.ack_peel node={} next_hop={} bytes={}",
                    crate::trace::node_id(),
                    next_hop,
                    ack_to_upstream.len()
                ),
                &upstream_chain,
                "relay.ack",
            );

            write_raw_frame(&mut upstream, &ack_to_upstream)
                .await
                .context("Failed relaying ACK upstream")
                .map_err(|e| {
                    push_packet_meta_trace(
                        "ComponentError",
                        ack_to_upstream.len(),
                        &format!(
                            "relay.write_ack_upstream ERROR node={} next_hop={} err={e:#}",
                            crate::trace::node_id(),
                            next_hop
                        ),
                        &upstream_chain,
                        "relay.error",
                    );
                    e
                })?;
            push_packet_meta_trace(
                "ComponentOutput",
                ack_to_upstream.len(),
                &format!(
                    "relay.ack_relay OUTPUT node={} next_hop={} bytes={}",
                    crate::trace::node_id(),
                    next_hop,
                    ack_to_upstream.len()
                ),
                &next_chain(&upstream_chain),
                "relay.output",
            );
        }
        None => {
            let terminal_chain = next_chain(&ingress_chain);
            let msg = format!(
                "relay.unexpected_terminal ERROR node={} bytes={} err=relay received next_hop=None; publisher must terminate the onion path",
                crate::trace::node_id(),
                payload_bytes.len()
            );
            push_packet_meta_trace(
                "ComponentError",
                payload_bytes.len(),
                &msg,
                &terminal_chain,
                "relay.error",
            );
            anyhow::bail!(msg);
        }
    }

    Ok(())
}

fn peel_ack_for_upstream(
    local_priv_key: &[u8; 32],
    ack_frame: &[u8],
) -> Option<(Vec<u8>, Option<String>)> {
    let opened = open(local_priv_key, ack_frame).ok()?;
    let layer: OnionLayer = serde_json::from_slice(&opened).ok()?;
    Some((layer.inner, layer.trace_id))
}

fn next_chain(parent: &str) -> String {
    let hop = crate::trace::next_hop_id();
    if parent.is_empty() {
        hop
    } else if hop.is_empty() {
        parent.to_string()
    } else {
        format!("{parent} -> {hop}")
    }
}
