use crate::ToplevelId;

#[derive(Debug)]
pub enum Event {
    Toplevel(ToplevelEvent),
}

/// Toplevel related events
#[derive(Debug)]
pub enum ToplevelEvent {
    /// A new toplevel was created.
    New(ToplevelId),

    /// The toplevel was closed.
    Closed(ToplevelId),
}
