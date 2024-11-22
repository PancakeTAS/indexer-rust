use super::{events, SharedState};

/// Handle a message from the websocket in parallel
pub fn handle_message(state: &SharedState, msg: String) -> anyhow::Result<()> {
    // parse event
    let event = events::parse_event(msg)?;

    // update cursor
    let time = match &event {
        events::Kind::CommitEvent { time_us, .. } => *time_us,
        events::Kind::IdentityEvent { time_us, .. } => *time_us,
        events::Kind::KeyEvent { time_us, .. } => *time_us
    };
    state.update_cursor(time);

    // TODO: metrics (plan to add proper metrics later)

    // TODO: forward to database
    // state.db.query/etc.

    Ok(())
}