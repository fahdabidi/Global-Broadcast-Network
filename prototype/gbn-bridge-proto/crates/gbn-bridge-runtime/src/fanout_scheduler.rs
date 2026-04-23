use gbn_bridge_protocol::BridgeData;

use crate::bridge_pool::BridgePool;
use crate::{RuntimeError, RuntimeResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FanoutSchedulerConfig {
    pub target_bridge_count: usize,
    pub reuse_timeout_ms: u64,
}

impl Default for FanoutSchedulerConfig {
    fn default() -> Self {
        Self {
            target_bridge_count: 10,
            reuse_timeout_ms: 1_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameDispatch {
    pub bridge_id: String,
    pub frame: BridgeData,
    pub reused_bridge: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FanoutPlan {
    pub initial: Vec<FrameDispatch>,
    pub pending: Vec<BridgeData>,
    pub reuse_triggered: bool,
}

#[derive(Debug, Clone)]
pub struct FanoutScheduler {
    config: FanoutSchedulerConfig,
    active_bridge_ids: Vec<String>,
    started_at_ms: u64,
}

impl FanoutScheduler {
    pub fn new(pool: &BridgePool, config: FanoutSchedulerConfig, started_at_ms: u64) -> Self {
        Self {
            config,
            active_bridge_ids: pool.selected_bridge_ids(),
            started_at_ms,
        }
    }

    pub fn initial_plan(&self, frames: &[BridgeData]) -> RuntimeResult<FanoutPlan> {
        if self.active_bridge_ids.is_empty() {
            return Err(RuntimeError::NoActiveUploadBridge);
        }

        if self.active_bridge_ids.len() >= self.config.target_bridge_count {
            return Ok(FanoutPlan {
                initial: frames
                    .iter()
                    .cloned()
                    .enumerate()
                    .map(|(index, frame)| FrameDispatch {
                        bridge_id: self.active_bridge_ids[index % self.config.target_bridge_count]
                            .clone(),
                        frame,
                        reused_bridge: false,
                    })
                    .collect(),
                pending: Vec::new(),
                reuse_triggered: false,
            });
        }

        let unique_count = self.active_bridge_ids.len().min(frames.len());
        let initial = frames
            .iter()
            .take(unique_count)
            .cloned()
            .enumerate()
            .map(|(index, frame)| FrameDispatch {
                bridge_id: self.active_bridge_ids[index].clone(),
                frame,
                reused_bridge: false,
            })
            .collect();
        let pending = frames.iter().skip(unique_count).cloned().collect();

        Ok(FanoutPlan {
            initial,
            pending,
            reuse_triggered: false,
        })
    }

    pub fn reuse_pending(
        &self,
        pending: &[BridgeData],
        now_ms: u64,
    ) -> RuntimeResult<Vec<FrameDispatch>> {
        if self.active_bridge_ids.is_empty() {
            return Err(RuntimeError::NoActiveUploadBridge);
        }

        if now_ms < self.started_at_ms + self.config.reuse_timeout_ms {
            return Ok(Vec::new());
        }

        Ok(pending
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, frame)| FrameDispatch {
                bridge_id: self.active_bridge_ids[index % self.active_bridge_ids.len()].clone(),
                frame,
                reused_bridge: true,
            })
            .collect())
    }

    pub fn mark_failed(&mut self, bridge_id: &str) -> bool {
        let before = self.active_bridge_ids.len();
        self.active_bridge_ids
            .retain(|candidate| candidate != bridge_id);
        before != self.active_bridge_ids.len()
    }

    pub fn reassign_frame(
        &self,
        frame: BridgeData,
        failed_bridge_id: &str,
    ) -> RuntimeResult<FrameDispatch> {
        let next_bridge = self
            .active_bridge_ids
            .iter()
            .find(|bridge_id| bridge_id.as_str() != failed_bridge_id)
            .cloned()
            .ok_or(RuntimeError::NoActiveUploadBridge)?;

        Ok(FrameDispatch {
            bridge_id: next_bridge,
            frame,
            reused_bridge: true,
        })
    }
}
