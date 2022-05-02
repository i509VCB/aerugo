use smithay::wayland::buffer::{Buffer, BufferHandler};

use super::State;

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &Buffer) {
        todo!()
    }
}
