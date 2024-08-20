use std::{marker::PhantomData, ops::Deref, sync::atomic::{AtomicUsize, Ordering}};

use crate::{EcsError, EcsResult};

// pub enum WriteLock {}

// impl LockKind for WriteLock {
//     const READ_ONLY: bool = false;
// }

// pub enum ReadLock {}

// impl LockKind for ReadLock {
//     const READ_ONLY: bool = true;
// }

// trait LockKind {
//     const READ_ONLY: bool;
// }

// pub struct LockGuard<'lock, T, K: LockKind> {
//     lock: &'lock PersistentLock<T>,
//     _marker: PhantomData<(T, K)>
// }

// impl<'lock, T, K: LockKind> Deref for LockGuard<'lock, T, K> {
//     type Target = ;

//     fn deref(&self) -> &Self::Target {
        
//     }
// }

// impl<'lock, T, K: LockKind> Drop for LockGuard<'lock, T, K> {
//     fn drop(&mut self) {
//         if K::READ_ONLY {
//             self.lock.release_read();
//         } else {
//             self.lock.release_write();
//         }
//     }
// }

pub struct PersistentLock {
    pub(crate) counter: AtomicUsize
}

impl PersistentLock {
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
