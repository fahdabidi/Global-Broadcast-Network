use ed25519_dalek::SigningKey;
use gbn_bridge_protocol::{
    publisher_identity, BridgeCapability, BridgeIngressEndpoint, BridgeRegister, PublicKeyBytes,
    ReachabilityClass, RefreshHintReason,
};
use gbn_bridge_publisher::PublisherAuthority;
use gbn_bridge_runtime::{
    CreatorConfig, CreatorRuntime, DiscoveryHint, DiscoveryHintSource, ExitBridgeConfig,
    ExitBridgeRuntime, InProcessPublisherClient, LocalHintSource, RuntimeError,
};

fn publisher_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[51_u8; 32])
}

fn publisher() -> PublisherAuthority {
    PublisherAuthority::new(publisher_signing_key())
}

fn node_public_key(seed: u8) -> PublicKeyBytes {
    publisher_identity(&SigningKey::from_bytes(&[seed; 32]))
}

fn bridge_register(
    bridge_id: &str,
    key_seed: u8,
    host: &str,
    udp_punch_port: u16,
) -> BridgeRegister {
    BridgeRegister {
        bridge_id: bridge_id.into(),
        identity_pub: node_public_key(key_seed),
        ingress_endpoints: vec![BridgeIngressEndpoint {
            host: host.into(),
            port: 443,
        }],
        requested_udp_punch_port: udp_punch_port,
        capabilities: vec![
            BridgeCapability::BootstrapSeed,
            BridgeCapability::CatalogRefresh,
            BridgeCapability::SessionRelay,
            BridgeCapability::BatchAssignment,
            BridgeCapability::ProgressReporting,
        ],
    }
}

fn bridge_config(
    bridge_id: &str,
    key_seed: u8,
    host: &str,
    udp_punch_port: u16,
) -> ExitBridgeConfig {
    ExitBridgeConfig {
        bridge_id: bridge_id.into(),
        identity_pub: node_public_key(key_seed),
        ingress_endpoint: BridgeIngressEndpoint {
            host: host.into(),
            port: 443,
        },
        requested_udp_punch_port: udp_punch_port,
        capabilities: vec![
            BridgeCapability::BootstrapSeed,
            BridgeCapability::CatalogRefresh,
            BridgeCapability::SessionRelay,
            BridgeCapability::BatchAssignment,
            BridgeCapability::ProgressReporting,
        ],
    }
}

fn startup_bridge(
    authority: PublisherAuthority,
    bridge_id: &str,
    key_seed: u8,
    host: &str,
    now_ms: u64,
) -> ExitBridgeRuntime {
    let client = InProcessPublisherClient::new(authority);
    let mut runtime = ExitBridgeRuntime::new(bridge_config(bridge_id, key_seed, host, 443), client);
    runtime.startup(ReachabilityClass::Direct, now_ms).unwrap();
    runtime
}

fn creator_runtime(creator_id: &str, key_seed: u8, host: &str) -> CreatorRuntime {
    CreatorRuntime::new(CreatorConfig {
        creator_id: creator_id.into(),
        ip_addr: host.into(),
        pub_key: node_public_key(key_seed),
        udp_punch_port: 443,
    })
}

#[test]
fn refresh_catalog_keeps_signed_non_direct_bridges_but_transport_stays_direct_only() {
    let mut authority = publisher();
    authority
        .register_bridge(
            bridge_register("bridge-brokered", 61, "198.51.100.20", 443),
            ReachabilityClass::Brokered,
            900,
        )
        .unwrap();
    authority
        .register_bridge(
            bridge_register("bridge-relay-only", 62, "198.51.100.21", 443),
            ReachabilityClass::RelayOnly,
            950,
        )
        .unwrap();

    let mut refresh_bridge = startup_bridge(authority, "bridge-direct", 60, "198.51.100.10", 1_000);
    let publisher_key = refresh_bridge.publisher_client().publisher_public_key();
    let mut creator = creator_runtime("creator-reachability-01", 70, "203.0.113.10");
    creator.load_publisher_trust_root(publisher_key).unwrap();

    let catalog = creator
        .refresh_catalog_via_bridge(&mut refresh_bridge, RefreshHintReason::Startup, 1_200)
        .unwrap();

    let bridge_ids: Vec<_> = catalog
        .bridges
        .iter()
        .map(|bridge| bridge.bridge_id.as_str())
        .collect();
    assert_eq!(
        bridge_ids,
        vec!["bridge-direct", "bridge-brokered", "bridge-relay-only"]
    );
    assert_eq!(
        creator
            .local_dht()
            .node("bridge-brokered")
            .unwrap()
            .reachability_class,
        ReachabilityClass::Brokered
    );
    assert_eq!(
        creator
            .local_dht()
            .node("bridge-relay-only")
            .unwrap()
            .reachability_class,
        ReachabilityClass::RelayOnly
    );

    let selected = creator.select_refresh_bridge(1_200).unwrap();
    assert_eq!(selected.bridge_id, "bridge-direct");

    let fanout = creator.begin_refresh_fanout(1_200).unwrap();
    assert_eq!(fanout.len(), 1);
    assert_eq!(fanout[0].target_node_id, "bridge-direct");
}

#[test]
fn signed_downgrade_clears_transport_state_and_blocks_weak_repromotion() {
    let authority = publisher();
    let mut refresh_bridge =
        startup_bridge(authority, "bridge-downgraded", 71, "198.51.100.30", 1_000);
    let publisher_key = refresh_bridge.publisher_client().publisher_public_key();
    let mut creator = creator_runtime("creator-reachability-02", 72, "203.0.113.20");
    creator.load_publisher_trust_root(publisher_key).unwrap();

    creator
        .refresh_catalog_via_bridge(&mut refresh_bridge, RefreshHintReason::Startup, 1_100)
        .unwrap();
    creator.mark_bridge_active("bridge-downgraded", 1_110);
    creator.ingest_weak_discovery_hints(&[DiscoveryHint {
        bridge_id: "bridge-downgraded".into(),
        host: "198.51.100.30".into(),
        port: 443,
        observed_at_ms: 1_120,
        source: DiscoveryHintSource::WeakDiscovery,
    }]);

    refresh_bridge
        .publisher_client_mut()
        .reclassify_bridge(
            "bridge-downgraded",
            ReachabilityClass::Brokered,
            Some(8443),
            1_200,
        )
        .unwrap();

    let refreshed = creator
        .refresh_catalog_via_bridge(&mut refresh_bridge, RefreshHintReason::ManualRefresh, 1_210)
        .unwrap();
    assert_eq!(refreshed.bridges.len(), 1);
    assert_eq!(
        refreshed.bridges[0].reachability_class,
        ReachabilityClass::Brokered
    );
    assert_eq!(refreshed.bridges[0].udp_punch_port, 8443);

    let node = creator.local_dht().node("bridge-downgraded").unwrap();
    assert_eq!(node.source, LocalHintSource::Catalog);
    assert_eq!(node.reachability_class, ReachabilityClass::Brokered);
    assert_eq!(node.udp_punch_port, 8443);
    assert_eq!(node.active_tunnel_since_ms, None);

    assert!(matches!(
        creator.select_refresh_bridge(1_210),
        Err(RuntimeError::NoUsableBridgeCandidate)
    ));
    assert!(matches!(
        creator.ordered_refresh_candidates(1_210),
        Err(RuntimeError::NoUsableBridgeCandidate)
    ));
}

#[test]
fn signed_direct_port_change_replaces_stale_local_port_state() {
    let authority = publisher();
    let mut refresh_bridge =
        startup_bridge(authority, "bridge-port-change", 81, "198.51.100.40", 1_000);
    let publisher_key = refresh_bridge.publisher_client().publisher_public_key();
    let mut creator = creator_runtime("creator-reachability-03", 82, "203.0.113.30");
    creator.load_publisher_trust_root(publisher_key).unwrap();

    creator
        .refresh_catalog_via_bridge(&mut refresh_bridge, RefreshHintReason::Startup, 1_100)
        .unwrap();
    creator.mark_bridge_active("bridge-port-change", 1_110);

    refresh_bridge
        .publisher_client_mut()
        .reclassify_bridge(
            "bridge-port-change",
            ReachabilityClass::Direct,
            Some(8443),
            1_200,
        )
        .unwrap();

    let refreshed = creator
        .refresh_catalog_via_bridge(&mut refresh_bridge, RefreshHintReason::ManualRefresh, 1_210)
        .unwrap();
    assert_eq!(refreshed.bridges.len(), 1);
    assert_eq!(refreshed.bridges[0].udp_punch_port, 8443);

    let node = creator.local_dht().node("bridge-port-change").unwrap();
    assert_eq!(node.reachability_class, ReachabilityClass::Direct);
    assert_eq!(node.udp_punch_port, 8443);
    assert_eq!(node.active_tunnel_since_ms, None);

    let selected = creator.select_refresh_bridge(1_210).unwrap();
    assert_eq!(selected.bridge_id, "bridge-port-change");
    assert_eq!(selected.udp_punch_port, 8443);

    let fanout = creator.begin_refresh_fanout(1_210).unwrap();
    assert_eq!(fanout.len(), 1);
    assert_eq!(fanout[0].target_node_id, "bridge-port-change");
    assert_eq!(fanout[0].target_udp_punch_port, 8443);
}
