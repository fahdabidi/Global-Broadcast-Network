use gbn_bridge_protocol::{BridgeCatalogResponse, PublicKeyBytes};

use crate::{RuntimeError, RuntimeResult};

#[derive(Debug, Clone, Default)]
pub struct CatalogCache {
    current: Option<BridgeCatalogResponse>,
}

impl CatalogCache {
    pub fn current(&self) -> Option<&BridgeCatalogResponse> {
        self.current.as_ref()
    }

    pub fn replace_verified(
        &mut self,
        response: BridgeCatalogResponse,
        publisher_key: &PublicKeyBytes,
        now_ms: u64,
    ) -> RuntimeResult<()> {
        response.verify_authority(publisher_key, now_ms)?;
        self.current = Some(response);
        Ok(())
    }

    pub fn load_valid(
        &self,
        publisher_key: &PublicKeyBytes,
        now_ms: u64,
    ) -> RuntimeResult<&BridgeCatalogResponse> {
        let response = self
            .current
            .as_ref()
            .ok_or(RuntimeError::CatalogUnavailable)?;
        response.verify_authority(publisher_key, now_ms)?;
        Ok(response)
    }

    pub fn clear(&mut self) {
        self.current = None;
    }
}
