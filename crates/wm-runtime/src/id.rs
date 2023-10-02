//! Id allocator

use std::{
    cell::RefCell,
    num::NonZeroU32,
    rc::{Rc, Weak},
};

use slotmap::SlotMap;

#[derive(Debug)]
pub enum AllocError {
    /// All ids the allocator can use have been exhausted.
    IdsExhausted,

    /// The id being freed is not in the allocator's range.
    OutOfRange,
}

/// Freelist based id allocator.
///
/// This allocator will allocate ids within a specified range at construction time and reuse lower ids before
/// higher ids.
#[derive(Debug)]
pub struct IdAllocator {
    next_free: Option<Rc<RefCell<Range>>>,
    allocs: SlotMap<RangeKey, Rc<RefCell<Range>>>,
    start: NonZeroU32,
    end: NonZeroU32,
}

impl IdAllocator {
    pub fn new(start: NonZeroU32, end: NonZeroU32) -> Self {
        assert!(start < end, "start id is less than end.");

        let mut allocs = SlotMap::with_key();
        let whole = allocs.insert_with_key(|key| {
            Rc::new(RefCell::new(Range {
                key,
                start,
                end,
                prev: None,
                next: None,
            }))
        });

        let range = allocs.get(whole).cloned().unwrap();

        Self {
            next_free: Some(range),
            allocs,
            start,
            end,
        }
    }

    pub fn alloc(&mut self) -> Result<NonZeroU32, AllocError> {
        let mut next_free = self.next_free.as_ref().ok_or(AllocError::IdsExhausted)?.borrow_mut();
        let id = next_free.start;

        if next_free.start != next_free.end {
            next_free.start = next_free.start.checked_add(1).expect("Handle overflow");
        } else {
            let Some(next) = next_free.next.as_ref().and_then(Weak::upgrade) else {
                // The last id was allocated.
                self.allocs.remove(next_free.key);
                drop(next_free);
                self.next_free.take();

                return Ok(id);
            };

            // Move to the next free range.
            self.allocs.remove(next_free.key);
            drop(next_free);
            self.next_free = Some(next);
        }

        Ok(id)
    }

    pub fn free(&mut self, id: NonZeroU32) -> Result<(), AllocError> {
        if id < self.start || id > self.end {
            return Err(AllocError::OutOfRange);
        }

        // No free ids are available, create a new range
        if self.next_free.is_none() {
            todo!();
            return Ok(());
        }

        // Find a contiguous range where the id could go
        let node =
            self.visit_node(|range| Some(range.start) == id.checked_add(1) || (range.end.checked_add(1)) == Some(id));

        match node {
            Some(node) => {
                let mut borrow = node.borrow_mut();

                if id < borrow.start && id < borrow.end {
                    borrow.start = id;
                } else {
                    borrow.end = id;
                }
            }

            None => {
                // new range
                todo!()
            }
        }

        Ok(())
    }

    fn visit_node<F: FnMut(&Range) -> bool>(&self, mut f: F) -> Option<Rc<RefCell<Range>>> {
        let mut next = self.next_free.as_ref().cloned();

        while let Some(range) = next.take() {
            let borrow = range.borrow();

            if f(&borrow) {
                drop(borrow);
                return Some(range);
            }

            next = borrow.next.as_ref().and_then(Weak::upgrade);
        }

        None
    }
}

slotmap::new_key_type! {
    struct RangeKey;
}

#[derive(Debug)]
struct Range {
    key: RangeKey,
    start: NonZeroU32,
    end: NonZeroU32,
    next: Option<Weak<RefCell<Range>>>,
    prev: Option<Weak<RefCell<Range>>>,
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use super::IdAllocator;

    #[test]
    fn alloc_contig() {
        let mut alloc = IdAllocator::new(NonZeroU32::MIN, NonZeroU32::MAX);

        let id = alloc.alloc().unwrap();
        assert_eq!(id.get(), 1);

        let id2 = alloc.alloc().unwrap();
        assert_eq!(id2.get(), 2);

        alloc.free(id2).unwrap();

        let id2 = alloc.alloc().unwrap();
        assert_eq!(id2.get(), 2);

        alloc.free(id2).unwrap();
        alloc.free(id).unwrap();

        let id = alloc.alloc().unwrap();
        assert_eq!(id.get(), 1);
    }

    #[test]
    fn alloc_disjoint() {
        let mut alloc = IdAllocator::new(NonZeroU32::MIN, NonZeroU32::MAX);

        let id = alloc.alloc().unwrap();
        assert_eq!(id.get(), 1);

        let id2 = alloc.alloc().unwrap();
        assert_eq!(id2.get(), 2);

        let id3 = alloc.alloc().unwrap();
        assert_eq!(id3.get(), 3);

        alloc.free(id2).unwrap();

        let id2 = alloc.alloc().unwrap();
        assert_eq!(id2.get(), 2);
    }
}
