use gbn_bridge_protocol::{
    BootstrapProgress, BridgeData, BridgeHeartbeat, BridgeLease, BridgeRegister, PublicKeyBytes,
    ReachabilityClass,
};
use gbn_bridge_publisher::{AuthorityResult, PublisherAuthority};

#[derive(Debug)]
pub struct InProcessPublisherClient {
    authority: PublisherAuthority,
    reported_progress: Vec<BootstrapProgress>,
    forwarded_frames: Vec<BridgeData>,
}

impl InProcessPublisherClient {
    pub fn new(authority: PublisherAuthority) -> Self {
        Self {
            authority,
            reported_progress: Vec::new(),
            forwarded_frames: Vec::new(),
        }
    }

    pub fn publisher_public_key(&self) -> PublicKeyBytes {
        self.authority.publisher_public_key().clone()
    }

    pub fn authority(&self) -> &PublisherAuthority {
        &self.authority
    }

    pub fn authority_mut(&mut self) -> &mut PublisherAuthority {
        &mut self.authority
    }

    pub fn replace_authority(&mut self, authority: PublisherAuthority) {
        self.authority = authority;
    }

    pub fn register_bridge(
        &mut self,
        request: BridgeRegister,
        reachability_class: ReachabilityClass,
        now_ms: u64,
    ) -> AuthorityResult<BridgeLease> {
        self.authority
            .register_bridge(request, reachability_class, now_ms)
    }

    pub fn renew_lease(&mut self, heartbeat: BridgeHeartbeat) -> AuthorityResult<BridgeLease> {
        self.authority.handle_heartbeat(heartbeat)
    }

    pub fn report_progress(&mut self, progress: BootstrapProgress) {
        self.reported_progress.push(progress);
    }

    pub fn reported_progress(&self) -> &[BootstrapProgress] {
        &self.reported_progress
    }

    pub fn forward_frame(&mut self, frame: BridgeData) {
        self.forwarded_frames.push(frame);
    }

    pub fn forwarded_frames(&self) -> &[BridgeData] {
        &self.forwarded_frames
    }
}
