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
            return Ok(());
        }

        //

        todo!()
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
