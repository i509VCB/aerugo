use smithay::{delegate_shm, wayland::shm::ShmState};

use super::Aerugo;

impl AsRef<ShmState> for Aerugo {
    fn as_ref(&self) -> &ShmState {
        &self.protocols.shm
    }
}

delegate_shm!(Aerugo);
