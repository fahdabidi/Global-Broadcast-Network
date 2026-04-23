//! Conduit ExitBridge runtime for registration, lease maintenance, punching, and seed bootstrap duties.

pub mod bootstrap_bridge;
pub mod bridge;
pub mod creator_listener;
pub mod forwarder;
pub mod heartbeat_loop;
pub mod lease_state;
pub mod progress_reporter;
pub mod publisher_client;
pub mod punch;

use gbn_bridge_protocol::{ProtocolError, ReachabilityClass};
use gbn_bridge_publisher::AuthorityError;
use thiserror::Error;

pub use bootstrap_bridge::{BootstrapBridgeState, SeedBridgeAssignment};
pub use bridge::{ExitBridgeConfig, ExitBridgeRuntime};
pub use creator_listener::CreatorListener;
pub use forwarder::{ForwardedFrame, PayloadForwarder};
pub use heartbeat_loop::HeartbeatLoop;
pub use lease_state::LeaseState;
pub use progress_reporter::ProgressReporter;
pub use publisher_client::InProcessPublisherClient;
pub use punch::{ActivePunchAttempt, PunchAuthorization, PunchManager};

pub type RuntimeResult<T> = Result<T, RuntimeError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum RuntimeError {
    #[error("bridge `{bridge_id}` has no active lease")]
    NoActiveLease { bridge_id: String },

    #[error("bridge `{bridge_id}` ingress is disabled")]
    IngressDisabled { bridge_id: String },

    #[error(
        "bridge `{bridge_id}` cannot expose ingress when reachability class is `{reachability_class:?}`"
    )]
    NonDirectReachability {
        bridge_id: String,
        reachability_class: ReachabilityClass,
    },

    #[error("publisher-directed or refresh-authorized punching required: {reason}")]
    PunchUnauthorized { reason: &'static str },

    #[error("bootstrap session `{bootstrap_session_id}` is not tracked by this bridge")]
    BootstrapSessionNotTracked { bootstrap_session_id: String },

    #[error(
        "punch attempt for bootstrap session `{bootstrap_session_id}` expired at `{attempt_expiry_ms}` before `{now_ms}`"
    )]
    PunchAttemptExpired {
        bootstrap_session_id: String,
        attempt_expiry_ms: u64,
        now_ms: u64,
    },

    #[error(
        "probe nonce mismatch for bootstrap session `{bootstrap_session_id}`: expected `{expected}`, got `{actual}`"
    )]
    ProbeNonceMismatch {
        bootstrap_session_id: String,
        expected: u64,
        actual: u64,
    },

    #[error("bridge `{bridge_id}` has no remembered reachability class for re-registration")]
    MissingReachabilityClass { bridge_id: String },

    #[error(transparent)]
    Authority(#[from] AuthorityError),

    #[error(transparent)]
    Protocol(#[from] ProtocolError),
}
