use std::num::NonZeroU32;

bitflags::bitflags! {
    #[derive(Debug, Default, Clone, Copy)]
    pub struct States: u32 {
        const MAXIMIZED = 0x01;
        const FULLSCREEN = 0x02;
        const RESIZING = 0x04;
        const ACTIVATED = 0x08;
        const TILED_LEFT = 0x10;
        const TILED_RIGHT = 0x20;
        const TILED_TOP = 0x40;
        const TILED_BOTTOM = 0x80;
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Decorations {
    #[default]
    Client,

    Server,
}

/// A configure describes an update to the state of a toplevel.
#[derive(Debug, Default, Clone)]
pub struct Configure {
    states: States,
    size: Option<(NonZeroU32, NonZeroU32)>,
    bounds: Option<(NonZeroU32, NonZeroU32)>,
    decorations: Decorations,
}

impl Configure {
    pub fn states(&mut self, states: States) -> &mut Self {
        self.states = states;
        self
    }

    pub fn size(&mut self, size: Option<(NonZeroU32, NonZeroU32)>) -> &mut Self {
        if let Some((width, height)) = size {
            assert!(width.get() <= MAX_I32, "width exceeds maximum size allowed by Wayland");
            assert!(
                height.get() <= MAX_I32,
                "height exceeds maximum size allowed by Wayland"
            );
        }

        self.size = size;
        self
    }

    pub fn bounds(&mut self, bounds: Option<(NonZeroU32, NonZeroU32)>) -> &mut Self {
        if let Some((width, height)) = bounds {
            assert!(width.get() <= MAX_I32, "width exceeds maximum size allowed by Wayland");
            assert!(
                height.get() <= MAX_I32,
                "height exceeds maximum size allowed by Wayland"
            );
        }

        self.bounds = bounds;
        self
    }

    pub fn decorations(&mut self, decorations: Decorations) -> &mut Self {
        self.decorations = decorations;
        self
    }
}

const MAX_I32: u32 = i32::MAX as u32;
