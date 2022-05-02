use smithay::{delegate_shm, wayland::shm::ShmState};

use super::State;

impl AsRef<ShmState> for State {
    fn as_ref(&self) -> &ShmState {
        &self.protocols.shm
    }
}

delegate_shm!(State);
