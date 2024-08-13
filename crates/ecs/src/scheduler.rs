use std::any::TypeId;
use std::collections::HashSet;
use std::sync::Arc;
use dashmap::{DashMap, DashSet};
use crate::{Component, EntityId, World};

#[derive(Default)]
pub struct Scheduler {
    /// Keeps track of entities that need to be despawned at the end of a tick.
    despawn_queue: DashSet<EntityId>,
    /// Keeps track of components to remove from entities at the end of a tick.
    remove_queue: DashMap<TypeId, HashSet<EntityId>>
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule_despawn(&self, entity: EntityId) {
        self.despawn_queue.insert(entity);
    }

    pub fn schedule_remove_component(&self, entity: EntityId, type_id: TypeId) {
        let mut entry = self.remove_queue
            .entry(type_id)
            .or_insert_with(HashSet::new);

        entry.value_mut().insert(entity);
    }

    pub fn pre_tick(&self, _world: &Arc<World>) {

    }

    pub fn post_tick(&self, world: &Arc<World>) {
        self.tick_removal(world);
        self.tick_despawn(world);
    }

    fn tick_removal(&self, world: &Arc<World>) {
        self.remove_queue.retain(|type_id, entities| {
            if let Some(store_kv) = world.components.map.get(type_id) {
                for entity in entities.iter() {
                    store_kv.value().remove(*entity);
                }
            }

            false
        });
    }

    fn tick_despawn(&self, world: &Arc<World>) {
        world.entities.free_many(self.despawn_queue.iter().map(|kv| *kv.key()));
        self.despawn_queue
            .retain(|entity| {
                world.components.despawn(*entity);
                false
            });
    }
}