use std::num::NonZeroU32;

use wayland_client::protocol::wl_surface::WlSurface;

use crate::{private, SurfaceNode, ToplevelId, ToplevelNode};

#[derive(Debug)]
pub struct Surface {
    pub generation: NonZeroU32,
    pub id: NonZeroU32,
    pub wl_surface: WlSurface,
}

impl Surface {}

impl private::NodePrivate for SurfaceNode {
    fn generation(&self) -> NonZeroU32 {
        self.0.generation
    }
}

#[derive(Debug)]
pub struct Toplevel {
    pub toplevel: ToplevelId,
}

impl Toplevel {}

impl private::NodePrivate for ToplevelNode {
    fn generation(&self) -> NonZeroU32 {
        self.0.toplevel.0.generation
    }
}
