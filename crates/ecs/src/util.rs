use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{EcsError, EcsResult};

pub struct LockMarker {
    pub(crate) counter: AtomicUsize
}

impl LockMarker {
    pub fn new() -> Self {
        Self { counter: AtomicUsize::new(0) }
    }

    pub fn acquire_read(&self) -> EcsResult<()> {
        if self.counter.load(Ordering::SeqCst) == usize::MAX {
            // Lock is already being used for writing.
            return Err(EcsError::StorageLocked("write lock active, cannot acquire read lock"))
        }

        self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(())        
    }

    pub fn release_read(&self) {
        self.counter.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn acquire_write(&self) -> EcsResult<()> {
        if self.counter.load(Ordering::SeqCst) != 0 {
            // Lock is already being used for reading.
            return Err(EcsError::StorageLocked("read or write lock active, cannot acquire write lock"))
        }

        self.counter.store(usize::MAX, Ordering::SeqCst);
        Ok(())
    }

    pub fn release_write(&self) {
        self.counter.store(0, Ordering::SeqCst);
    }
}
