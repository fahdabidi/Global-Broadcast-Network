//! # MCN Chunker
//!
//! Splits a sanitized video file into fixed-size chunks and generates a
//! BLAKE3 content-addressed manifest.
//!
//! ## Design
//!
//! - Streaming: reads the file in `chunk_size` increments (default 1MB)
//! - Only 1 chunk is held in memory at a time (target: <50MB peak for 4GB files)
//! - Last chunk is padded to `chunk_size` to prevent chunk-size traffic analysis
//! - Each chunk receives a BLAKE3 hash for post-decryption integrity verification
//!
//! ## Output
//!
//! - `Vec<Chunk>`: ordered list of plaintext chunk byte vectors
//! - `ChunkManifest`: metadata listing all chunk hashes, sizes, and session info

// TODO: Implement in Phase 1 execution
// - chunk_video(input_path, chunk_size) -> Result<(Vec<Vec<u8>>, ChunkManifest)>
// - chunk_video_streaming(input_path, chunk_size, callback) -> Result<ChunkManifest>
