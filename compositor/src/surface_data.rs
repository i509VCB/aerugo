use std::any::Any;

use smithay::{
    backend::renderer::buffer_dimensions,
    reexports::wayland_server::protocol::wl_buffer::WlBuffer,
    utils::{Logical, Physical, Size},
    wayland::compositor::{BufferAssignment, SurfaceAttributes},
};

/// State of an attached buffer.
#[derive(Debug)]
pub struct AttachedBufferState {
    pub buffer: WlBuffer,
    pub texture: Option<Box<dyn Any + 'static>>,
    pub dimensions: Option<Size<i32, Physical>>,
    pub scale: i32,
}

/// Surface data available on a WlSurface and any of it's children.
#[derive(Debug, Default)]
pub struct SurfaceData {
    pub attached_buffer_state: Option<AttachedBufferState>,
}

impl SurfaceData {
    pub(crate) fn update_buffer(&mut self, attributes: &mut SurfaceAttributes) {
        match attributes.buffer.take() {
            Some(BufferAssignment::NewBuffer { buffer, .. }) => {
                let dimensions = buffer_dimensions(&buffer);

                let new_state = AttachedBufferState {
                    buffer,
                    texture: None,
                    dimensions,
                    scale: attributes.buffer_scale,
                };

                if let Some(old) = self.attached_buffer_state.replace(new_state) {
                    old.buffer.release();
                }
            }

            Some(BufferAssignment::Removed) => {
                // Drop the current attached buffer state since the buffer is no longer attached.
                self.attached_buffer_state.take();
            }

            None => (),
        }
    }

    pub fn size(&self) -> Option<Size<i32, Logical>> {
        if let Some(state) = &self.attached_buffer_state {
            state.dimensions.map(|size| size.to_logical(state.scale))
        } else {
            None
        }
    }
}
