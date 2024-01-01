use crate::component::{Components, SpawnBundle};
use crate::entity::{Entities, Entity, EntityId};
use crate::{NakedSys, Sys, SysContainer, Systems};
use std::sync::Arc;
use crate::scheduler::Scheduler;

pub struct World {
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    pub(crate) systems: Systems,
    pub(crate) scheduler: Scheduler
}

impl World {
    pub fn new() -> Arc<World> {
        Arc::new(World::default())
    }

    pub fn spawn<B: SpawnBundle>(self: &Arc<Self>, bundle: B) -> Entity {
        let entity = self.entities.alloc();
        bundle.insert_into(&self.components, entity);

        Entity {
            world: Arc::clone(self),
            id: entity,
        }
    }

    #[inline]
    pub fn spawn_empty(self: &Arc<Self>) -> Entity {
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

    pub async fn execute(self: &Arc<Self>) {
        self.systems.call(self).await;
    }
}

impl Default for World {
    fn default() -> World {
        World {
            entities: Entities::default(),
            components: Components::default(),
            systems: Systems::default(),
            scheduler: Scheduler::default()
        }
    }
}
