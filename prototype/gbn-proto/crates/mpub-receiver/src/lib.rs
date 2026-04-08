//! # MPub Receiver
//!
//! Publisher-side component that receives encrypted chunks from the MCN relay
//! network, buffers them, performs ECDH key derivation, decrypts each chunk,
//! verifies BLAKE3 integrity, and reassembles the original video.
//!
//! ## Design
//!
//! - Listens on multiple TCP ports (one per simulated relay path)
//! - Buffers incoming encrypted chunks keyed by (session_id, chunk_index)
//! - Handles out-of-order arrival gracefully — reassembly uses manifest order
//! - Decrypts each chunk independently (Chunk-Then-Encrypt architecture)
//! - Verifies BLAKE3(decrypted_chunk) matches the manifest hash
//! - Strips padding from the final chunk
//! - Writes reassembled video to the staging area
//!
//! ## Integrity Guarantees
//!
//! 1. AES-256-GCM auth tag: detects any bit flip in ciphertext (tamper rejection)
//! 2. BLAKE3 hash: confirms decrypted content matches the Creator's original chunk
//! 3. Full-file SHA-256: end-to-end verification after complete reassembly

// TODO: Implement in Phase 1 execution
// - start_receiver(ports, publisher_privkey) -> ReceiverHandle
// - await_complete_session(receiver) -> ReassembledVideo
// - verify_reassembly(original_hash, reassembled_path) -> bool
