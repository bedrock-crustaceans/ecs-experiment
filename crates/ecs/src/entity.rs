use crate::{filter::FilterParams, Component, World};
use std::any::TypeId;
use std::fmt::Debug;
use std::iter::FusedIterator;
use std::marker::PhantomData;
use std::sync::Arc;
use bitvec::vec::BitVec;
use parking_lot::{RwLock, RwLockReadGuard};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(pub(crate) usize);

#[derive(Clone)]
pub struct Entity<'wb, 'w> {
    pub(crate) world: &'wb World<'w>,
    pub(crate) id: EntityId,
}

impl<'wb, 'w> Entity<'wb, 'w> {
    pub fn id(&self) -> EntityId {
        self.id
    }

    /// Despawns the entity, invalidating its ID and removing all its components from storage.
    /// The actual change is only performed after all systems have completed.
    pub fn despawn(self) {
        self.world.scheduler.schedule_despawn(self.id);
    }

    pub fn has<T: Component>(&self) -> bool {
        self.world.components.has_component::<T>(self.id)
    }

    // pub fn get<T: Component>(&self) -> Option<&T> {
    //     self.world.components.get(self.id)
    // }

    // pub fn get_mut<T: Component>(&self) -> Option<&mut T> {
    //     self.world.components.get_mut(self.id)
    // }

    /// Removes a component from an entity. The actual change is only performed
    /// after all systems have completed running in order to prevent issues.
    ///
    /// If the entity did not have this component in the first place this does nothing.
    pub fn remove<T: Component>(&self) {
        let type_id = TypeId::of::<T>();
        self.world.scheduler.schedule_remove_component(self.id, type_id);
    }
}

#[derive(Default)]
pub(crate) struct Entities {
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

    pub fn free_many<I: Iterator<Item = EntityId>>(&self, iter: I) {
        let mut lock = self.indices.write();
        for entity in iter {
            lock.set(entity.0, false);
        }
    }

    pub fn iter<'wb, 'w, F: FilterParams>(&'wb self, world: &'wb World<'w>) -> EntityIter<'wb, 'w, F> {
        let entities = self.indices.read();
        EntityIter {
            world, entities, iter_index: 0, _marker: PhantomData
        }
    }
}

pub(crate) struct EntityIter<'wb, 'w, F>
where
    F: FilterParams
{
    pub world: &'wb World<'w>,
    pub entities: RwLockReadGuard<'wb, BitVec>,
    pub iter_index: usize,
    pub _marker: PhantomData<&'wb F>
}

impl<'wb, 'w, F> Iterator for EntityIter<'wb, 'w, F>
where
    F: FilterParams
{
    type Item = Entity<'wb, 'w>;

    fn next(&mut self) -> Option<Self::Item> {
        // Use a loop rather than recursion for cache reasons.
        loop {
            let next_id = self.entities
                .iter_ones()
                .nth(self.iter_index)?;

            self.iter_index += 1;
            let entity = Entity {
                world: self.world.clone(),
                id: EntityId(next_id)
            };

            if F::filter(&entity) {
                break Some(entity)
            }
        }
    }
}

impl<'wb, 'w, F: FilterParams> FusedIterator for EntityIter<'wb, 'w, F> {}