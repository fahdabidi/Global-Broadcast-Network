use gbn_bridge_protocol::BridgeControlCommand;

use crate::assignment;
use crate::control::ControlSessionState;
use crate::{AuthorityResult, PublisherAuthority};

pub fn dispatch_pending_commands(
    authority: &mut PublisherAuthority,
    session: &ControlSessionState,
    sent_at_ms: u64,
) -> AuthorityResult<Vec<BridgeControlCommand>> {
    let pending = authority.pending_bridge_commands(&session.bridge_id);
    let mut commands = Vec::with_capacity(pending.len());
    for record in pending {
        authority.mark_bridge_command_dispatched(
            &session.bridge_id,
            &record.command_id,
            sent_at_ms,
        )?;
        commands.push(assignment::wire_command(&session.session_id, &record));
    }
    Ok(commands)
}
