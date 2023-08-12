use std::{
    collections::BTreeMap,
    num::NonZeroU32,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::Setup;

static GENERATION: AtomicU32 = AtomicU32::new(1);

pub struct Inner {
    /// Generation of this wm instance. This is retrieved from the global generation counter.
    generation: NonZeroU32,

    /// The next surface id.
    next_surface_id: NonZeroU32,

    /// The next toplevel id.
    next_toplevel_id: NonZeroU32,

    /// All toplevel instances known by this wm.
    toplevels: BTreeMap<NonZeroU32, ToplevelInfo>,
    // TODO:
    // - surfaces
    // - transactions
}

impl Inner {
    // TODO: new
    pub fn new() -> Result<Self, Setup> {
        let generation = GENERATION.fetch_add(1, Ordering::AcqRel);

        // If 0 is loaded, there have been billions of dead or failed instances. Clearly this is something we
        // can't really deal with.
        let generation = NonZeroU32::new(generation).expect("Internal generation counter overflowed");

        let next_surface_id = NonZeroU32::new(1).unwrap();
        let next_toplevel_id = NonZeroU32::new(1).unwrap();
        let toplevels = BTreeMap::new();

        let inner = Self {
            generation,
            next_surface_id,
            next_toplevel_id,
            toplevels,
        };

        Ok(inner)
    }
}

#[derive(Debug)]
pub struct ToplevelInfo {}
