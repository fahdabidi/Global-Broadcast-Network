use std::{
    io::Write,
    time::Duration,
};
use tempfile::NamedTempFile;

use mcn_crypto::{create_upload_session, generate_publisher_keypair};
use mcn_chunker::{chunk_file, hash_file};
use mcn_router_sim::create_multipath_router;
use mpub_receiver::Receiver;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn test_full_pipeline_small_video() {
    let (pub_secret, pub_key) = generate_publisher_keypair();
    
    // 2. Create test "video" (5 MB random data)
    let content: Vec<u8> = (0u8..=255).cycle().take(5 * 1024 * 1024).collect();
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(&content).unwrap();
    f.flush().unwrap();
    let original_path = f.path();

    // 3. Hash original
    let original_hash = hash_file(&original_path).unwrap();

    // 4. Chunk
    let chunk_size = 1024 * 1024; // 1MB
    let (chunks, manifest) = chunk_file(&original_path, chunk_size).unwrap();
    assert_eq!(manifest.total_chunks, 5);

    // 5. Crypto Session
    let session = create_upload_session(&pub_key, manifest.total_chunks, original_hash).unwrap();

    // 6. Receiver Setup
    let receiver = Receiver::new(vec![
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
        "127.0.0.1:0".parse().unwrap(),
    ]);
    let mut receiver_handle = receiver.start().await.unwrap();

    // 7. Router Setup (3 paths, 2 hops)
    let router = create_multipath_router(receiver_handle.bound_addrs.clone(), 2, 10, 50).await.unwrap();

    // 8. Encrypt and Route
    for (i, data) in chunks.iter().enumerate() {
        let info = &manifest.chunks[i];
        let packet = session.encrypt_chunk(info.index, data, info.hash).unwrap();
        router.send_chunk(&packet).await.unwrap();
    }

    // 9. Reassemble and Verify
    let completed = receiver_handle.await_session(manifest.session_id, Duration::from_secs(5)).await.unwrap();
    let out = NamedTempFile::new().unwrap();
    completed.decrypt_and_reassemble(out.path(), &pub_secret, &session.session_init, &manifest).unwrap();
    
    let is_match = completed.verify(original_hash, out.path()).unwrap();
    assert!(is_match, "Reassembled file must exactly match the original");

    // Cleanup
    router.shutdown().await;
    receiver_handle.shutdown();
}
