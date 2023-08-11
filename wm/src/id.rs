//! Internal id types

use std::num::{NonZeroU32, NonZeroU64};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Toplevel {
    pub generation: NonZeroU32,
    pub id: NonZeroU32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transaction {
    pub generation: NonZeroU32,
    pub id: NonZeroU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Node {
    Toplevel(Toplevel),
    // TODO: How to represent a surface
}
