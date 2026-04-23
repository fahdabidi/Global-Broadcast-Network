use crate::lease_state::LeaseState;

#[derive(Debug, Clone, Default)]
pub struct CreatorListener {
    ingress_exposed: bool,
    last_state_change_ms: Option<u64>,
}

impl CreatorListener {
    pub fn refresh_from_lease(&mut self, lease_state: &LeaseState, now_ms: u64) {
        let should_expose = lease_state.ingress_allowed(now_ms);
        if should_expose != self.ingress_exposed {
            self.ingress_exposed = should_expose;
            self.last_state_change_ms = Some(now_ms);
        }
    }

    pub fn disable(&mut self, now_ms: u64) {
        if self.ingress_exposed {
            self.ingress_exposed = false;
            self.last_state_change_ms = Some(now_ms);
        }
    }

    pub fn is_exposed(&self) -> bool {
        self.ingress_exposed
    }

    pub fn last_state_change_ms(&self) -> Option<u64> {
        self.last_state_change_ms
    }
}
