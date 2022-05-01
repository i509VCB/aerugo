use smithay::reexports::wayland_server::Display;

use crate::state::Hihiirokane;

#[derive(Debug)]
pub struct CalloopData {
    display: Display<Hihiirokane>,
    state: Hihiirokane,
}
