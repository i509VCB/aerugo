#[derive(Debug)]
pub enum Event {
    Toplevel(ToplevelEvent),
}

/// Toplevel related events
#[derive(Debug)]
pub enum ToplevelEvent {}
