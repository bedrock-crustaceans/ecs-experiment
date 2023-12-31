use std::sync::Arc;
use crate::component::Components;
use crate::entity::{Entities, EntityId, EntityMut};
use crate::{NakedSys, SpawnBundle, Sys, SysContainer, Systems};

pub struct World {
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    systems: Systems,
}

impl World {
    pub fn new() -> Arc<World> {
        Arc::new(World::default())
    }

    pub fn spawn<B: SpawnBundle>(&self, bundle: B) -> EntityMut {
        let entity = self.entities.alloc();
        bundle.insert_into(&self.components, entity);

        EntityMut {
            world: self,
            id: entity,
        }
    }

    #[inline]
    pub fn spawn_empty(&self) -> EntityMut {
        self.spawn(())
    }

    pub fn despawn(&self, entity: EntityId) {
        self.components.despawn(entity);
    }

    pub fn system<P, S>(&self, system: S)
    where
        P: Send + Sync + 'static,
        S: NakedSys<P> + Send + Sync + 'static,
        SysContainer<P, S>: Sys,
    {
        let wrapped = Arc::new(system.into_container());
        self.systems.push(wrapped);
    }

    pub fn execute(self: &Arc<Self>) {
        self.systems.call(self);
    }
}

impl Default for World {
    fn default() -> World {
        World {
            entities: Entities::default(),
            components: Components::default(),
            systems: Systems::default(),
        }
    }
}
