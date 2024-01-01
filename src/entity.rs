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

    /// Removes a component from an entity. The actual change is only performed
    /// after all systems have completed running in order to prevent issues.
    ///
    /// If the entity did not have this component in the first place nothing will happen.
    pub fn remove<T: Component>(&self) {
        let type_id = TypeId::of::<T>();
        self.world.scheduler.remove_component(self.id, type_id);
    }
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
