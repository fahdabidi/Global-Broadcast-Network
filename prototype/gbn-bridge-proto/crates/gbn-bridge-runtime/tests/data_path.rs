use ed25519_dalek::SigningKey;
use gbn_bridge_protocol::{
    publisher_identity, BridgeAckStatus, BridgeCapability, BridgeCatalogRequest,
    BridgeIngressEndpoint, PublicKeyBytes, ReachabilityClass,
};
use gbn_bridge_publisher::PublisherAuthority;
use gbn_bridge_runtime::{
    AckTracker, BridgePool, ChunkSender, ChunkSenderConfig, CreatorConfig, CreatorRuntime,
    ExitBridgeConfig, ExitBridgeRuntime, FanoutScheduler, FanoutSchedulerConfig, FrameDispatch,
    FramePayloadConfig, InProcessPublisherClient, UploadSessionConfig,
};

fn publisher_signing_key() -> SigningKey {
    SigningKey::from_bytes(&[55_u8; 32])
}

fn publisher() -> PublisherAuthority {
    PublisherAuthority::new(publisher_signing_key())
}

fn node_public_key(seed: u8) -> PublicKeyBytes {
    publisher_identity(&SigningKey::from_bytes(&[seed; 32]))
}

fn bridge_config(bridge_id: &str, key_seed: u8, host: &str) -> ExitBridgeConfig {
    ExitBridgeConfig {
        bridge_id: bridge_id.into(),
        identity_pub: node_public_key(key_seed),
        ingress_endpoint: BridgeIngressEndpoint {
            host: host.into(),
            port: 443,
        },
        requested_udp_punch_port: 443,
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
    bridge_id: &str,
    key_seed: u8,
    host: &str,
    shared_client: &InProcessPublisherClient,
    now_ms: u64,
) -> ExitBridgeRuntime {
    let mut runtime = ExitBridgeRuntime::new(
        bridge_config(bridge_id, key_seed, host),
        shared_client.clone(),
    );
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

fn prime_creator(
    creator: &mut CreatorRuntime,
    bridge_for_catalog: &mut ExitBridgeRuntime,
    now_ms: u64,
) {
    creator
        .load_publisher_trust_root(bridge_for_catalog.publisher_client().publisher_public_key())
        .unwrap();
    let catalog = bridge_for_catalog
        .publisher_client_mut()
        .issue_catalog(
            &BridgeCatalogRequest {
                creator_id: creator.config().creator_id.clone(),
                known_catalog_id: None,
                direct_only: true,
                refresh_hint: None,
            },
            now_ms,
        )
        .unwrap();
    creator.ingest_catalog(catalog.clone(), now_ms).unwrap();
    for bridge in &catalog.bridges {
        creator.mark_bridge_active(&bridge.bridge_id, now_ms);
    }
}

#[test]
fn end_to_end_upload_through_single_bridge_with_reuse_timeout() {
    let shared_client = InProcessPublisherClient::new(publisher());
    let mut bridge = startup_bridge("bridge-a", 61, "198.51.100.10", &shared_client, 1_000);
    let mut creator = creator_runtime("creator-upload-01", 71, "203.0.113.10");
    prime_creator(&mut creator, &mut bridge, 1_100);

    let pool = BridgePool::from_creator(&creator, 10).unwrap();
    assert_eq!(pool.selected_bridge_ids(), vec!["bridge-a".to_string()]);

    let mut sender = ChunkSender::default();
    let payload = b"opaque-conduit-payload-through-one-bridge".to_vec();
    let session = sender.begin_session(&creator, &payload, 2_000).unwrap();
    let mut tracker = AckTracker::new(&session);
    let scheduler = FanoutScheduler::new(
        &pool,
        FanoutSchedulerConfig {
            target_bridge_count: 10,
            reuse_timeout_ms: 50,
        },
        2_000,
    );

    sender
        .open_selected_bridges(
            &session,
            &pool.selected_bridge_ids(),
            std::slice::from_mut(&mut bridge),
            2_000,
        )
        .unwrap();

    let plan = scheduler.initial_plan(session.frames()).unwrap();
    assert_eq!(plan.initial.len(), 1);
    assert_eq!(plan.pending.len(), session.frame_count() - 1);

    let mut acks = sender
        .send_dispatches(
            &plan.initial,
            std::slice::from_mut(&mut bridge),
            &mut tracker,
            2_010,
        )
        .unwrap();
    let reused = scheduler.reuse_pending(&plan.pending, 2_060).unwrap();
    assert!(reused.iter().all(|dispatch| dispatch.reused_bridge));
    acks.extend(
        sender
            .send_dispatches(
                &reused,
                std::slice::from_mut(&mut bridge),
                &mut tracker,
                2_060,
            )
            .unwrap(),
    );
    sender
        .close_selected_bridges(
            &session,
            &pool.selected_bridge_ids(),
            std::slice::from_mut(&mut bridge),
            2_090,
        )
        .unwrap();

    assert!(tracker.all_acked());
    assert!(acks
        .iter()
        .any(|ack| ack.status == BridgeAckStatus::Complete));

    let session_record = bridge
        .publisher_client()
        .authority()
        .upload_session(session.session_id())
        .unwrap()
        .clone();
    assert_eq!(
        session_record.frames_by_sequence.len(),
        session.frame_count()
    );
    assert_eq!(
        session_record.frames_by_sequence[&0].via_bridge_id,
        "bridge-a"
    );
    assert_eq!(
        bridge.local_forwarded_frames()[0].frame.ciphertext,
        session_record.frames_by_sequence[&0].frame.ciphertext
    );
}

#[test]
fn publisher_acks_are_correlated_to_the_correct_sessions() {
    let shared_client = InProcessPublisherClient::new(publisher());
    let mut bridge = startup_bridge("bridge-a", 62, "198.51.100.11", &shared_client, 1_000);

    let mut creator_a = creator_runtime("creator-a", 72, "203.0.113.11");
    let mut creator_b = creator_runtime("creator-b", 73, "203.0.113.12");
    prime_creator(&mut creator_a, &mut bridge, 1_100);
    prime_creator(&mut creator_b, &mut bridge, 1_150);

    let pool_a = BridgePool::from_creator(&creator_a, 10).unwrap();
    let pool_b = BridgePool::from_creator(&creator_b, 10).unwrap();
    let mut sender = ChunkSender::default();

    let session_a = sender.begin_session(&creator_a, b"alpha", 2_000).unwrap();
    let session_b = sender.begin_session(&creator_b, b"beta", 2_010).unwrap();

    sender
        .open_selected_bridges(
            &session_a,
            &pool_a.selected_bridge_ids(),
            std::slice::from_mut(&mut bridge),
            2_000,
        )
        .unwrap();
    sender
        .open_selected_bridges(
            &session_b,
            &pool_b.selected_bridge_ids(),
            std::slice::from_mut(&mut bridge),
            2_010,
        )
        .unwrap();

    let dispatch_a = FrameDispatch {
        bridge_id: "bridge-a".into(),
        frame: session_a.frames()[0].clone(),
        reused_bridge: false,
    };
    let dispatch_b = FrameDispatch {
        bridge_id: "bridge-a".into(),
        frame: session_b.frames()[0].clone(),
        reused_bridge: false,
    };

    let mut tracker_a = AckTracker::new(&session_a);
    let mut tracker_b = AckTracker::new(&session_b);
    let ack_a = sender
        .send_dispatches(
            &[dispatch_a],
            std::slice::from_mut(&mut bridge),
            &mut tracker_a,
            2_020,
        )
        .unwrap()
        .pop()
        .unwrap();
    let ack_b = sender
        .send_dispatches(
            &[dispatch_b],
            std::slice::from_mut(&mut bridge),
            &mut tracker_b,
            2_021,
        )
        .unwrap()
        .pop()
        .unwrap();

    assert_eq!(ack_a.session_id, session_a.session_id());
    assert_eq!(ack_b.session_id, session_b.session_id());
}

#[test]
fn duplicate_bridge_data_is_acked_as_duplicate_and_not_reingested() {
    let shared_client = InProcessPublisherClient::new(publisher());
    let mut bridge = startup_bridge("bridge-a", 63, "198.51.100.12", &shared_client, 1_000);
    let mut creator = creator_runtime("creator-dup", 74, "203.0.113.13");
    prime_creator(&mut creator, &mut bridge, 1_100);

    let pool = BridgePool::from_creator(&creator, 10).unwrap();
    let mut sender = ChunkSender::with_config(ChunkSenderConfig {
        upload_session: UploadSessionConfig {
            frame_payload: FramePayloadConfig {
                frame_size_bytes: 4,
            },
        },
    });
    let session = sender
        .begin_session(&creator, b"duplicate-check", 2_000)
        .unwrap();
    sender
        .open_selected_bridges(
            &session,
            &pool.selected_bridge_ids(),
            std::slice::from_mut(&mut bridge),
            2_000,
        )
        .unwrap();

    let frame = session.frames()[0].clone();
    let dispatch = FrameDispatch {
        bridge_id: "bridge-a".into(),
        frame: frame.clone(),
        reused_bridge: false,
    };
    let mut tracker = AckTracker::new(&session);
    let first_ack = sender
        .send_dispatches(
            &[dispatch.clone()],
            std::slice::from_mut(&mut bridge),
            &mut tracker,
            2_010,
        )
        .unwrap()
        .pop()
        .unwrap();
    assert_eq!(first_ack.status, BridgeAckStatus::Accepted);

    let duplicate_ack = bridge.forward_session_frame(frame, 2_011).unwrap();
    assert_eq!(duplicate_ack.status, BridgeAckStatus::Duplicate);

    let session_record = bridge.publisher_client();
    let authority = session_record.authority();
    let session_record = authority.upload_session(session.session_id()).unwrap();
    assert_eq!(session_record.frames_by_sequence.len(), 1);
}

#[test]
fn mid_session_failover_reassigns_pending_frames_to_another_bridge() {
    let shared_client = InProcessPublisherClient::new(publisher());
    let mut bridge_a = startup_bridge("bridge-a", 64, "198.51.100.13", &shared_client, 1_000);
    let bridge_b = startup_bridge("bridge-b", 65, "198.51.100.14", &shared_client, 1_000);

    let mut creator = creator_runtime("creator-failover", 75, "203.0.113.14");
    prime_creator(&mut creator, &mut bridge_a, 1_100);
    creator.mark_bridge_active("bridge-b", 1_100);

    let pool = BridgePool::from_creator(&creator, 2).unwrap();
    let mut sender = ChunkSender::with_config(ChunkSenderConfig {
        upload_session: UploadSessionConfig {
            frame_payload: FramePayloadConfig {
                frame_size_bytes: 4,
            },
        },
    });
    let session = sender
        .begin_session(&creator, b"failover-path-uses-next-bridge", 2_000)
        .unwrap();
    let mut tracker = AckTracker::new(&session);
    let mut scheduler = FanoutScheduler::new(
        &pool,
        FanoutSchedulerConfig {
            target_bridge_count: 2,
            reuse_timeout_ms: 50,
        },
        2_000,
    );

    let selected = pool.selected_bridge_ids();
    let mut bridges = [bridge_a, bridge_b];
    sender
        .open_selected_bridges(&session, &selected, &mut bridges, 2_000)
        .unwrap();

    let plan = scheduler.initial_plan(session.frames()).unwrap();
    assert!(plan.initial.len() >= 3);
    let first_two: Vec<_> = plan.initial.iter().take(2).cloned().collect();
    sender
        .send_dispatches(&first_two, &mut bridges, &mut tracker, 2_010)
        .unwrap();

    let failing_dispatch = plan
        .initial
        .iter()
        .find(|dispatch| dispatch.bridge_id == "bridge-a" && dispatch.frame.sequence > 0)
        .unwrap()
        .clone();
    scheduler.mark_failed("bridge-a");
    let reassigned = scheduler
        .reassign_frame(failing_dispatch.frame.clone(), "bridge-a")
        .unwrap();
    assert_eq!(reassigned.bridge_id, "bridge-b");
    sender
        .send_dispatches(&[reassigned], &mut bridges, &mut tracker, 2_030)
        .unwrap();

    let session_record = bridges[0]
        .publisher_client()
        .authority()
        .upload_session(session.session_id())
        .unwrap()
        .clone();
    assert_eq!(
        session_record.frames_by_sequence[&2].via_bridge_id,
        "bridge-b"
    );
}
