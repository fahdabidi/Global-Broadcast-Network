use std::env;
use std::fs;
use std::path::PathBuf;

use ed25519_dalek::SigningKey;

use crate::storage::{StorageError, StorageResult};

pub mod kms;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublisherSigningSource {
    Hex(String),
    File(PathBuf),
    Kms(kms::KmsSigningKeyConfig),
}

impl PublisherSigningSource {
    pub fn from_env() -> StorageResult<Self> {
        match env::var("GBN_BRIDGE_PUBLISHER_SIGNING_MODE")
            .unwrap_or_else(|_| "hex".to_string())
            .to_lowercase()
            .as_str()
        {
            "hex" => {
                if let Ok(value) = env::var("GBN_BRIDGE_PUBLISHER_SIGNING_KEY_HEX") {
                    Ok(Self::Hex(value))
                } else {
                    Ok(Self::Hex("09".repeat(32)))
                }
            }
            "file" => {
                let path = env::var("GBN_BRIDGE_PUBLISHER_SIGNING_KEY_FILE").map_err(|_| {
                    StorageError::Config(
                        "GBN_BRIDGE_PUBLISHER_SIGNING_KEY_FILE is required when signing mode=file"
                            .into(),
                    )
                })?;
                Ok(Self::File(PathBuf::from(path)))
            }
            "kms" => Ok(Self::Kms(kms::KmsSigningKeyConfig::from_env()?)),
            other => Err(StorageError::Config(format!(
                "unsupported publisher signing mode {other:?}; expected hex, file, or kms"
            ))),
        }
    }

    pub fn load_signing_key(&self) -> StorageResult<SigningKey> {
        match self {
            Self::Hex(value) => decode_hex_32(value).map(|bytes| SigningKey::from_bytes(&bytes)),
            Self::File(path) => {
                let value = fs::read_to_string(path)
                    .map_err(|error| StorageError::Backend(error.to_string()))?;
                decode_hex_32(&value).map(|bytes| SigningKey::from_bytes(&bytes))
            }
            Self::Kms(config) => kms::KmsSigningKeyLoader::new(config.clone()).load_signing_key(),
        }
    }
}

fn decode_hex_32(value: &str) -> StorageResult<[u8; 32]> {
    let trimmed = value.trim();
    if trimmed.len() != 64 {
        return Err(StorageError::Config(format!(
            "publisher signing key must contain exactly 64 hex characters, got {}",
            trimmed.len()
        )));
    }

    let mut bytes = [0_u8; 32];
    for (index, chunk) in trimmed.as_bytes().chunks(2).enumerate() {
        let pair = std::str::from_utf8(chunk).map_err(|_| {
            StorageError::Config("publisher signing key must be valid utf-8".into())
        })?;
        bytes[index] = u8::from_str_radix(pair, 16)
            .map_err(|_| StorageError::Config(format!("invalid signing-key hex byte {pair:?}")))?;
    }

    Ok(bytes)
}
