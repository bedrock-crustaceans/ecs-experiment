use crate::component::TypedStorage;
use crate::{Component, QueryParams, World};
use std::any::TypeId;
use std::fmt::{Debug, Display};
use std::marker::PhantomData;
use std::num::NonZeroUsize;
use std::ops::Deref;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use bitvec::vec::BitVec;
use parking_lot::RwLock;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(pub(crate) usize);

pub struct Entity {
    pub(crate) world: Arc<World>,
    pub(crate) id: EntityId,
}

impl Entity {
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

#[derive(Default)]
pub struct Entities {
    indices: RwLock<BitVec>
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }

    pub fn alloc(&self) -> EntityId {
        let gap = self.indices
            .read()
            .iter()
            .by_vals()
            .enumerate()
            .find_map(|(i, v)| if v { None } else { Some(i) });

        let id = if let Some(gap) = gap {
            self.indices.write().set(gap, true);

            gap
        } else {
            let mut lock = self.indices.write();
            let len = lock.len();
            lock.push(true);

            len
        };

        EntityId(id)
    }

    pub fn free(&self, entity: EntityId) {
        self.indices.write().set(entity.0, false);
    }
}
