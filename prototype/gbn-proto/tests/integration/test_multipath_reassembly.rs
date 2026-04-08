//! Multipath reassembly tests:
//! Validate that chunks arriving via different paths and in random order
//! are correctly reassembled into the original video.

#[tokio::test]
async fn test_reverse_order_reassembly() {
    // TODO: Deliver chunks in reverse order (N, N-1, ..., 1, 0) → verify SHA-256 match
}

#[tokio::test]
async fn test_random_order_reassembly() {
    // TODO: Shuffle chunk delivery order randomly → verify SHA-256 match
}

#[tokio::test]
async fn test_multipath_with_jitter() {
    // TODO: Send chunks across 5 independent paths with 50-500ms random jitter → verify match
}

#[tokio::test]
async fn test_multipath_one_slow_path() {
    // TODO: One path has 2000ms delay; others have 50ms → verify all chunks arrive, match
}
