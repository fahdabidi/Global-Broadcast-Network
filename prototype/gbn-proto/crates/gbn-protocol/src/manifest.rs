//! Content manifest types for GDS publication (Phase 2).
//!
//! Defined here in Phase 1 so the protocol boundary is established early,
//! even though these types are not used until Phase 2.

use serde::{Deserialize, Serialize};

use crate::chunk::ContentId;
use crate::crypto::PublisherPublicKey;

/// Signed content manifest published to the GDS DHT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentManifest {
    /// BLAKE3 hash of the full plaintext video.
    pub content_id: ContentId,
    /// Publisher's Ed25519 public key.
    pub publisher_id: PublisherPublicKey,
    /// Unix timestamp of publication.
    pub publication_timestamp: u64,

    // -- Editorial metadata --
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    pub duration_secs: u32,
    pub thumbnail_cid: Option<ContentId>,

    // -- Storage parameters --
    pub storage: StorageParams,

    /// Ed25519 signature covering all fields above.
    pub signature: Vec<u8>,
}

/// Reed-Solomon storage parameters within a content manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageParams {
    /// Number of data shards required for reconstruction.
    pub rs_k: u8,
    /// Total number of shards (data + parity).
    pub rs_n: u8,
    /// Size of each GDS chunk in bytes before erasure coding.
    pub chunk_size_bytes: u32,
    /// Total number of GDS-level chunks.
    pub total_chunks: u32,
    /// BLAKE3 content IDs of all n shards, in order.
    pub shard_cids: Vec<ContentId>,
    /// AES-256 content key, encrypted with the Publisher's X25519 public key.
    pub content_key_encrypted: Vec<u8>,
}
