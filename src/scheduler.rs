use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::Mutex;
use crate::{Component, EntityId, World};

#[derive(Default)]
pub struct Scheduler {
    /// When a component is removed from an entity inside of a system, the storage is locked
    /// and therefore cannot be modified. This means that removing the component immediately
    /// would cause a deadlock, therefore this removal operation queue exists which destroys
    /// all components after the systems have finished.
    removal_queue: DashMap<TypeId, HashSet<EntityId>>
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remove_component(&self, entity: EntityId, type_id: TypeId) {
        let mut entry = self.removal_queue
            .entry(type_id)
            .or_insert_with(HashSet::new);

        entry.value_mut().insert(entity);
    }

    pub fn pre_tick(&self, _world: &Arc<World>) {

    }

    pub fn post_tick(&self, world: &Arc<World>) {
        self.tick_removal(world);
    }

    fn tick_removal(&self, world: &Arc<World>) {
        self.removal_queue.retain(|type_id, entities| {
            if let Some(store_kv) = world.components.map.get(type_id) {
                for entity in entities.iter() {
                    store_kv.value().remove(*entity);
                }
            }

            false
        });
    }
}