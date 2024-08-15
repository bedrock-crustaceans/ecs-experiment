use crate::component::{Components, SpawnBundle};
use crate::entity::{Entities, Entity};
use crate::{Events, ParameterizedSystem, Resource, Resources, System, SystemContainer, SystemParams, Systems};
use std::sync::Arc;
use crate::scheduler::Scheduler;

#[derive(Default)]
pub struct World {
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    pub(crate) systems: Systems,
    pub(crate) scheduler: Scheduler,
    pub(crate) events: Events,
    pub(crate) resources: Resources
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

    pub fn add_resource<R: Resource>(self: &Arc<Self>, resource: R) {
        self.resources.insert(resource)
    }

    pub fn add_system<P, S>(self: &Arc<Self>, system: S)
    where
        P: SystemParams + Send + Sync + 'static,
        S: ParameterizedSystem<P> + Send + Sync + 'static,
        SystemContainer<P, S>: System,
    {
        let wrapped = Arc::new(system.into_container());
        self.systems.push(self, wrapped);
    }

    pub async fn tick(self: &Arc<Self>) {
        self.scheduler.pre_tick(self);
        self.systems.call(self).await;
        self.scheduler.post_tick(self);
    }
}
