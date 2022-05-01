use smithay::{delegate_shm, wayland::shm::ShmState};

use super::Hihiirokane;

impl AsRef<ShmState> for Hihiirokane {
    fn as_ref(&self) -> &ShmState {
        &self.protocols.shm
    }
}

delegate_shm!(Hihiirokane);
