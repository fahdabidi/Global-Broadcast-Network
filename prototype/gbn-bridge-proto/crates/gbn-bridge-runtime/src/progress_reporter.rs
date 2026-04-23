use gbn_bridge_protocol::{BootstrapProgress, BootstrapProgressStage};

use crate::publisher_client::InProcessPublisherClient;

#[derive(Debug, Clone, Default)]
pub struct ProgressReporter {
    emitted: Vec<BootstrapProgress>,
}

impl ProgressReporter {
    pub fn report(
        &mut self,
        publisher_client: &mut InProcessPublisherClient,
        reporter_id: &str,
        bootstrap_session_id: &str,
        stage: BootstrapProgressStage,
        active_bridge_count: u16,
        reported_at_ms: u64,
    ) {
        let progress = BootstrapProgress {
            bootstrap_session_id: bootstrap_session_id.to_string(),
            reporter_id: reporter_id.to_string(),
            stage,
            active_bridge_count,
            reported_at_ms,
        };

        publisher_client.report_progress(progress.clone());
        self.emitted.push(progress);
    }

    pub fn emitted(&self) -> &[BootstrapProgress] {
        &self.emitted
    }
}
