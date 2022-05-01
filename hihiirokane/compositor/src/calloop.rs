use smithay::reexports::wayland_server::Display;

use crate::state::Hihiirokane;

#[derive(Debug)]
pub struct CalloopData {
    pub display: Display<Hihiirokane>,
    pub state: Hihiirokane,
}
