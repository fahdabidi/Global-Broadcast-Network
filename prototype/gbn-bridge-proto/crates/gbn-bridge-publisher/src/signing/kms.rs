use std::env;

use ed25519_dalek::SigningKey;

use crate::storage::{StorageError, StorageResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KmsSigningKeyConfig {
    pub key_id: String,
    pub mock_key_hex: String,
}

impl KmsSigningKeyConfig {
    pub fn from_env() -> StorageResult<Self> {
        let key_id = env::var("GBN_BRIDGE_PUBLISHER_KMS_KEY_ID").map_err(|_| {
            StorageError::Config(
                "GBN_BRIDGE_PUBLISHER_KMS_KEY_ID is required when signing mode=kms".into(),
            )
        })?;
        let mock_key_hex = env::var("GBN_BRIDGE_PUBLISHER_KMS_MOCK_KEY_HEX").map_err(|_| {
            StorageError::Config(
                "GBN_BRIDGE_PUBLISHER_KMS_MOCK_KEY_HEX is required for local kms-mode validation"
                    .into(),
            )
        })?;
        Ok(Self {
            key_id,
            mock_key_hex,
        })
    }
}

#[derive(Debug, Clone)]
pub struct KmsSigningKeyLoader {
    config: KmsSigningKeyConfig,
}

impl KmsSigningKeyLoader {
    pub fn new(config: KmsSigningKeyConfig) -> Self {
        Self { config }
    }

    pub fn load_signing_key(&self) -> StorageResult<SigningKey> {
        let bytes = decode_hex_32(&self.config.mock_key_hex)?;
        Ok(SigningKey::from_bytes(&bytes))
    }
}

fn decode_hex_32(value: &str) -> StorageResult<[u8; 32]> {
    let trimmed = value.trim();
    if trimmed.len() != 64 {
        return Err(StorageError::Config(format!(
            "kms mock signing key must contain exactly 64 hex characters, got {}",
            trimmed.len()
        )));
    }

    let mut bytes = [0_u8; 32];
    for (index, chunk) in trimmed.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk)
            .map_err(|_| StorageError::Config("kms mock signing key must be valid utf-8".into()))?;
        bytes[index] = u8::from_str_radix(pair, 16).map_err(|_| {
            StorageError::Config(format!("invalid kms mock signing-key hex byte {pair:?}"))
        })?;
    }

    Ok(bytes)
}
