use crate::{Configure, ToplevelId, Wm};

pub struct Transaction<'wm> {
    _wm: &'wm Wm,
}

impl<'wm> Transaction<'wm> {
    pub fn dependency(&mut self, transaction: &Transaction<'wm>) {}

    pub fn configure(&mut self, toplevel: ToplevelId, configure: Configure) {}
}
