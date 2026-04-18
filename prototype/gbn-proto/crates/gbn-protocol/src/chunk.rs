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
    #[serde(with = "crate::serde_b64::fixed_32")]
    pub hash: ContentId,
    /// Size of the plaintext chunk in bytes (before encryption).
    pub size: u32,
}

/// Manifest describing all chunks in an upload session.
/// Created by the MCN chunker, transmitted (encrypted) to the Publisher.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkManifest {
    /// Unique session identifier.
    #[serde(with = "crate::serde_b64::fixed_16")]
    pub session_id: SessionId,
    /// Total number of chunks in this upload.
    pub total_chunks: u32,
    /// BLAKE3 hash of the complete sanitized video file.
    #[serde(with = "crate::serde_b64::fixed_32")]
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
    #[serde(with = "crate::serde_b64::fixed_16")]
    pub session_id: SessionId,
    /// Zero-indexed position in the video sequence.
    pub chunk_index: u32,
    /// Total number of chunks (so receiver knows when complete).
    pub total_chunks: u32,
    /// BLAKE3 hash of the plaintext (for post-decryption verification).
    #[serde(with = "crate::serde_b64::fixed_32")]
    pub plaintext_hash: ContentId,
    /// The GCM nonce used for this chunk (nonce_base XOR chunk_index).
    #[serde(with = "crate::serde_b64::fixed_12")]
    pub nonce: [u8; 12],
    /// AES-256-GCM ciphertext (includes auth tag appended by aes-gcm crate).
    #[serde(with = "crate::serde_b64::bytes")]
    pub ciphertext: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct LegacyEncryptedChunkPacket {
        session_id: SessionId,
        chunk_index: u32,
        total_chunks: u32,
        plaintext_hash: ContentId,
        nonce: [u8; 12],
        ciphertext: Vec<u8>,
    }

    #[test]
    fn encrypted_chunk_packet_round_trips_and_shrinks_json() {
        let packet = EncryptedChunkPacket {
            session_id: [0x11; 16],
            chunk_index: 7,
            total_chunks: 9,
            plaintext_hash: [0x22; 32],
            nonce: [0x33; 12],
            ciphertext: (0u8..=255).cycle().take(512).collect(),
        };

        let encoded = serde_json::to_vec(&packet).unwrap();
        let decoded: EncryptedChunkPacket = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.session_id, packet.session_id);
        assert_eq!(decoded.chunk_index, packet.chunk_index);
        assert_eq!(decoded.total_chunks, packet.total_chunks);
        assert_eq!(decoded.plaintext_hash, packet.plaintext_hash);
        assert_eq!(decoded.nonce, packet.nonce);
        assert_eq!(decoded.ciphertext, packet.ciphertext);

        let legacy = LegacyEncryptedChunkPacket {
            session_id: packet.session_id,
            chunk_index: packet.chunk_index,
            total_chunks: packet.total_chunks,
            plaintext_hash: packet.plaintext_hash,
            nonce: packet.nonce,
            ciphertext: packet.ciphertext.clone(),
        };
        let legacy_encoded = serde_json::to_vec(&legacy).unwrap();

        assert!(
            encoded.len() < legacy_encoded.len() / 2,
            "expected compact encoding to materially reduce packet JSON: compact={} legacy={}",
            encoded.len(),
            legacy_encoded.len()
        );
    }
}
