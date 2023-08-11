use std::{
    num::NonZeroU32,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::Setup;

static GENERATION: AtomicU32 = AtomicU32::new(1);

pub struct Inner {
    pub generation: NonZeroU32,
}

impl Inner {
    // TODO: new
    pub fn new() -> Result<Self, Setup> {
        let generation = GENERATION.fetch_add(1, Ordering::AcqRel);

        // If 0 is loaded, there have been billions of dead or failed instances. Clearly this is something we
        // can't really deal with.
        let generation = NonZeroU32::new(generation).expect("Internal generation counter overflowed");

        let inner = Self { generation };

        Ok(inner)
    }
}
