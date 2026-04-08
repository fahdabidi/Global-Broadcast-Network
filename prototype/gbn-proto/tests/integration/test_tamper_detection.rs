//! Tamper detection tests:
//! Verify that AES-256-GCM authentication rejects any modification to ciphertext.

#[test]
fn test_single_bit_flip_detected() {
    // TODO: Encrypt a chunk, flip 1 bit in ciphertext, attempt decrypt → must fail
}

#[test]
fn test_chunk_swap_detected() {
    // TODO: Swap ciphertext of chunk 5 and chunk 10 (keeping headers) → both must fail
}

#[test]
fn test_wrong_publisher_key_rejected() {
    // TODO: Encrypt with Publisher A's key, decrypt with Publisher B → must fail for ALL chunks
}

#[test]
fn test_cross_session_isolation() {
    // TODO: Two sessions to same Publisher → chunks from session A cannot decrypt with session B key
}
