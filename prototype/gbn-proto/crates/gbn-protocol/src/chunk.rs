//! Chunk-related protocol types shared across MCN and MPub.

use serde::{Deserialize, Serialize};

/// A unique identifier for an upload session.
/// Generated fresh by the Creator for each upload.
pub type SessionId = [u8; 16];

/// A BLAKE3 hash used as a content identifier.
pub type ContentId = [u8; 32];

/// Metadata for a single chunk within an upload session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// Zero-indexed position in the video sequence.
    pub index: u32,
    /// BLAKE3 hash of the plaintext chunk data.
    pub hash: ContentId,
    /// Size of the plaintext chunk in bytes (before encryption).
    pub size: u32,
}

/// Manifest describing all chunks in an upload session.
/// Created by the MCN chunker, transmitted (encrypted) to the Publisher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkManifest {
    /// Unique session identifier.
    pub session_id: SessionId,
    /// Total number of chunks in this upload.
    pub total_chunks: u32,
    /// BLAKE3 hash of the complete sanitized video file.
    pub content_hash: ContentId,
    /// Total size of the sanitized video in bytes.
    pub total_size: u64,
    /// Per-chunk metadata in sequence order.
    pub chunks: Vec<ChunkInfo>,
}

/// An encrypted chunk ready for network transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedChunkPacket {
    /// Links this chunk to its upload session.
    pub session_id: SessionId,
    /// Zero-indexed position in the video sequence.
    pub chunk_index: u32,
    /// Total number of chunks (so receiver knows when complete).
    pub total_chunks: u32,
    /// BLAKE3 hash of the plaintext (for post-decryption verification).
    pub plaintext_hash: ContentId,
    /// The GCM nonce used for this chunk (nonce_base XOR chunk_index).
    pub nonce: [u8; 12],
    /// AES-256-GCM ciphertext (includes auth tag appended by aes-gcm crate).
    pub ciphertext: Vec<u8>,
}
