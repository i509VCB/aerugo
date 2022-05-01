use smithay::wayland::buffer::{Buffer, BufferHandler};

use super::Hihiirokane;

impl BufferHandler for Hihiirokane {
    fn buffer_destroyed(&mut self, _buffer: &Buffer) {
        todo!()
    }
}
