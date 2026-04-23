use gbn_bridge_protocol::{BridgeDescriptor, PublicKeyBytes, ReachabilityClass};

use crate::local_dht::LocalDhtNode;

pub fn is_creator_ingress_capable(reachability_class: &ReachabilityClass) -> bool {
    matches!(reachability_class, ReachabilityClass::Direct)
}

pub fn is_transport_eligible_bridge(bridge: &BridgeDescriptor) -> bool {
    is_creator_ingress_capable(&bridge.reachability_class) && !bridge.ingress_endpoints.is_empty()
}

pub fn preserve_active_tunnel(
    existing: Option<&LocalDhtNode>,
    next_ip_addr: &str,
    next_pub_key: &PublicKeyBytes,
    next_udp_punch_port: u16,
    next_reachability_class: &ReachabilityClass,
) -> Option<u64> {
    let existing = existing?;
    if !is_creator_ingress_capable(&existing.reachability_class)
        || !is_creator_ingress_capable(next_reachability_class)
    {
        return None;
    }

    if existing.ip_addr != next_ip_addr
        || &existing.pub_key != next_pub_key
        || existing.udp_punch_port != next_udp_punch_port
    {
        return None;
    }

    existing.active_tunnel_since_ms
}
