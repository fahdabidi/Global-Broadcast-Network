use anyhow::Result;
use aws_sdk_servicediscovery::types::HealthStatusFilter;
use crate::gossip::{
    new_plumtree_behaviour, GossipRequest, GossipResponse, OutboundGossip, PlumTreeBehaviour,
    PlumTreeEngine,
};
use libp2p::futures::StreamExt;
use libp2p::{
    identity,
    kad::{store::MemoryStore, Behaviour as Kademlia, Config as KademliaConfig},
    multiaddr::Protocol,
    noise,
    request_response,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Swarm, SwarmBuilder,
};
use std::{env, net::IpAddr};
use std::time::Duration;

#[derive(NetworkBehaviour)]
pub struct RouterBehaviour {
    pub kademlia: Kademlia<MemoryStore>,
    pub gossip: PlumTreeBehaviour,
}

#[derive(Debug, Clone)]
pub struct GossipRuntime {
    pub engine: PlumTreeEngine,
}

pub fn gossip_config_from_env() -> (usize, usize) {
    let gossip_bps = env::var("GBN_GOSSIP_BPS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(15 * 1024 * 1024 / 8);
    let max_tracked_messages = env::var("GBN_MAX_TRACKED_MESSAGES")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(10_000);
    (gossip_bps, max_tracked_messages)
}

impl GossipRuntime {
    pub fn from_env() -> Self {
        let (gossip_bps, max_tracked_messages) = gossip_config_from_env();
        Self {
            engine: PlumTreeEngine::new(gossip_bps, max_tracked_messages),
        }
    }
}

pub async fn build_swarm(local_key: identity::Keypair) -> Result<Swarm<RouterBehaviour>> {
    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let peer_id = key.public().to_peer_id();
            let mut kad_config = KademliaConfig::default();
            // Faster queries for the simulated environment
            kad_config.set_query_timeout(Duration::from_secs(5));
            let store = MemoryStore::new(peer_id);

            RouterBehaviour {
                kademlia: Kademlia::with_config(peer_id, store, kad_config),
                gossip: new_plumtree_behaviour(),
            }
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    bootstrap_from_cloudmap(&mut swarm).await?;

    Ok(swarm)
}

fn send_outbound(
    swarm: &mut Swarm<RouterBehaviour>,
    outbound: impl IntoIterator<Item = OutboundGossip>,
) {
    for msg in outbound {
        swarm
            .behaviour_mut()
            .gossip
            .send_request(&msg.peer, msg.request);
    }
}

pub fn handle_gossip_event(
    swarm: &mut Swarm<RouterBehaviour>,
    runtime: &mut GossipRuntime,
    event: request_response::Event<GossipRequest, GossipResponse>,
) {
    if let request_response::Event::Message { peer, message } = event {
        match message {
            request_response::Message::Request {
                request,
                channel,
                ..
            } => {
                let outbound = runtime.engine.on_request(peer, request);
                send_outbound(swarm, outbound);
                let _ = swarm
                    .behaviour_mut()
                    .gossip
                    .send_response(channel, GossipResponse::Ack);
            }
            request_response::Message::Response { .. } => {}
        }
    }
}

pub async fn drive_swarm_once(
    swarm: &mut Swarm<RouterBehaviour>,
    runtime: &mut GossipRuntime,
) -> Result<()> {
    if let Some(event) = swarm.next().await {
        match event {
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                runtime.engine.add_lazy_peer(peer_id);
            }
            SwarmEvent::Behaviour(RouterBehaviourEvent::Gossip(event)) => {
                handle_gossip_event(swarm, runtime, event);
            }
            _ => {}
        }
    }
    Ok(())
}

pub async fn bootstrap_from_cloudmap(swarm: &mut Swarm<RouterBehaviour>) -> Result<usize> {
    let namespace = match env::var("GBN_CLOUDMAP_NAMESPACE") {
        Ok(v) if !v.is_empty() => v,
        _ => return Ok(0),
    };
    let service_name = env::var("GBN_CLOUDMAP_SERVICE_NAME").unwrap_or_else(|_| "relay".to_string());
    let p2p_port = env::var("GBN_P2P_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(4001);

    let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
    let client = aws_sdk_servicediscovery::Client::new(&config);

    let instances = client
        .discover_instances()
        .namespace_name(&namespace)
        .service_name(&service_name)
        .health_status(HealthStatusFilter::Healthy)
        .send()
        .await?;

    let mut added = 0usize;
    for instance in instances.instances() {
        let Some(attrs) = instance.attributes() else { continue };
        let ip: Option<String> = attrs.get("AWS_INSTANCE_IPV4").cloned();
        let peer_id_str: Option<String> = attrs.get("GBN_PEER_ID").cloned();

        let Some(ip) = ip else { continue };
        let Ok(ip_addr) = ip.parse::<IpAddr>() else { continue };

        let mut addr = libp2p::Multiaddr::empty();
        addr.push(Protocol::from(ip_addr));
        addr.push(Protocol::Tcp(p2p_port));

        if let Some(peer_id_str) = peer_id_str {
            if let Ok(peer_id) = peer_id_str.parse::<libp2p::PeerId>() {
                swarm.behaviour_mut().kademlia.add_address(&peer_id, addr.clone());
                added += 1;
            }
        }

        let _ = swarm.dial(addr);
    }

    Ok(added)
}
