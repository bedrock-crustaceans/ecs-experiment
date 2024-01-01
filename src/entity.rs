use crate::component::TypedStorage;
use crate::{Component, QueryBundle, World};
use std::any::TypeId;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(pub(crate) NonZeroUsize);

// pub struct StorageLockReadGuard<'world, T: Component> {
//     pub(crate) world: &'world World,
//     pub(crate) _marker: PhantomData<&'world T>
// }
//
// impl<T: Component + Debug> Debug for StorageLockReadGuard<'_, T> {
//     fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
//         self.deref().fmt(fmt)
//     }
// }
//
// impl<T: Component + Display> Display for StorageLockReadGuard<'_, T> {
//     fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
//         self.deref().fmt(fmt)
//     }
// }
//
// impl<T: Component> Deref for StorageLockReadGuard<'_, T> {
//     type Target = T;
//
//     fn deref(&self) -> &Self::Target {
//         // SAFETY: When this value exists, the storage is locked by this thread and therefore it is
//         // safe to create a read guard.
//         let lock = unsafe {
//
//         };
//     }
// }
//
// impl<T: Component> Drop for StorageLockReadGuard<'_, T> {
//     fn drop(&mut self) {
//         let type_id = TypeId::of::<T>();
//         let store_kv = self.world.components.map.get(&type_id).unwrap();
//         let store = store_kv.value();
//         let typed_store: &TypedStorage<T> = store
//             .as_any()
//             .downcast_ref()
//             .unwrap();
//
//         unsafe {
//             typed_store.storage.force_unlock_read()
//         }
//     }
// }

pub struct Entity<'world> {
    pub(crate) world: &'world World,
    pub(crate) id: EntityId,
}

impl<'world> Entity<'world> {
    pub fn id(&self) -> EntityId {
        self.id
    }

    pub fn despawn(self) {
        self.world.components.despawn(self.id);
    }

    pub fn get<T: Component>(&self) -> Option<&T> {
        self.world.components.get(self.id)
    }

    pub fn get_mut<T: Component>(&self) -> Option<&mut T> {
        self.world.components.get_mut(self.id)
    }

    // pub fn get<T: Component>(&self) -> Option<StorageLockReadGuard<'world, T>> {
    //     let type_id = TypeId::of::<T>();
    //     let store_kv = self.world.components.map.get(&type_id)?;
    //     let store = store_kv.value();
    //
    //     let typed_store: &TypedStorage<T> = store
    //         .as_any()
    //         .downcast_ref()
    //         .unwrap();
    //
    //     let lock = typed_store.storage.read();
    //     std::mem::forget(lock);
    //
    //     Some(StorageLockReadGuard {
    //         world: self.world,
    //         _marker: PhantomData
    //     })
    // }

    // pub fn get<T: QueryBundle>(&self) -> Option<StorageLockReadGuard<'world, T>> {
    //     let type_id = TypeId::of::<T>();
    //
    //     let store = self.world.components.map.get(&type_id)?.value();
    //     let typed_store: &TypedStorage<T> = store
    //         .as_any()
    //         .downcast_ref()
    //         .unwrap();
    //
    //     let lock = typed_store.storage.read();
    //     std::mem::forget(lock);
    //
    //     Some(StorageLockReadGuard {
    //         world: self.world
    //     })
    // }

    // pub fn get_mut<T: Component>(&self) -> Option<&mut T> {
    //     let type_id = TypeId::of::<T>();
    //
    //     let store = self.world.components.map.get(&type_id)?.value();
    //     let typed_store: &TypedStorage<T> = store
    //         .as_any()
    //         .downcast_ref()
    //         .unwrap();
    //
    //     typed_store.fetch_mut(self.id)
    // }
}

pub struct Entities {
    next_index: AtomicUsize,
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }

    pub fn alloc(&self) -> EntityId {
        EntityId(NonZeroUsize::new(self.next_index.fetch_add(1, Ordering::Relaxed)).unwrap())
    }
}

impl Default for Entities {
    fn default() -> Entities {
        Entities {
            next_index: AtomicUsize::new(1),
        }
    }
}
