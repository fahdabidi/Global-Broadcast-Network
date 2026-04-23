use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

use gbn_bridge_protocol::{
    BootstrapProgress, BridgeAck, BridgeCatalogRequest, BridgeCatalogResponse, BridgeClose,
    BridgeData, BridgeHeartbeat, BridgeLease, BridgeOpen, BridgeRegister, CreatorJoinRequest,
    PublicKeyBytes, ReachabilityClass,
};
use gbn_bridge_publisher::{AuthorityBootstrapPlan, AuthorityResult, PublisherAuthority};

#[derive(Debug, Clone)]
pub struct InProcessPublisherClient {
    authority: Rc<RefCell<PublisherAuthority>>,
    reported_progress: Vec<BootstrapProgress>,
    forwarded_frames: Vec<BridgeData>,
}

impl InProcessPublisherClient {
    pub fn new(authority: PublisherAuthority) -> Self {
        Self {
            authority: Rc::new(RefCell::new(authority)),
            reported_progress: Vec::new(),
            forwarded_frames: Vec::new(),
        }
    }

    pub fn publisher_public_key(&self) -> PublicKeyBytes {
        self.authority.borrow().publisher_public_key().clone()
    }

    pub fn authority(&self) -> Ref<'_, PublisherAuthority> {
        self.authority.borrow()
    }

    pub fn authority_mut(&self) -> RefMut<'_, PublisherAuthority> {
        self.authority.borrow_mut()
    }

    pub fn replace_authority(&mut self, authority: PublisherAuthority) {
        *self.authority.borrow_mut() = authority;
    }

    pub fn register_bridge(
        &mut self,
        request: BridgeRegister,
        reachability_class: ReachabilityClass,
        now_ms: u64,
    ) -> AuthorityResult<BridgeLease> {
        self.authority
            .borrow_mut()
            .register_bridge(request, reachability_class, now_ms)
    }

    pub fn renew_lease(&mut self, heartbeat: BridgeHeartbeat) -> AuthorityResult<BridgeLease> {
        self.authority.borrow_mut().handle_heartbeat(heartbeat)
    }

    pub fn issue_catalog(
        &mut self,
        request: &BridgeCatalogRequest,
        now_ms: u64,
    ) -> AuthorityResult<BridgeCatalogResponse> {
        self.authority.borrow_mut().issue_catalog(request, now_ms)
    }

    pub fn begin_bootstrap(
        &mut self,
        request: CreatorJoinRequest,
        now_ms: u64,
    ) -> AuthorityResult<AuthorityBootstrapPlan> {
        self.authority.borrow_mut().begin_bootstrap(request, now_ms)
    }

    pub fn open_bridge_session(&mut self, open: BridgeOpen) -> AuthorityResult<()> {
        self.authority.borrow_mut().open_bridge_session(open)
    }

    pub fn ingest_bridge_frame(
        &mut self,
        via_bridge_id: &str,
        frame: BridgeData,
        received_at_ms: u64,
    ) -> AuthorityResult<BridgeAck> {
        self.authority
            .borrow_mut()
            .ingest_bridge_frame(via_bridge_id, frame, received_at_ms)
    }

    pub fn close_bridge_session(&mut self, close: BridgeClose) -> AuthorityResult<()> {
        self.authority.borrow_mut().close_bridge_session(close)
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
