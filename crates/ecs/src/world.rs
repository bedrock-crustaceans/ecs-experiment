use crate::component::{Components, SpawnBundle};
use crate::entity::{Entities, Entity};
use crate::{Events, ParameterizedSystem, Resource, Resources, System, SystemContainer, SystemParams, Systems};
use std::sync::Arc;
use crate::scheduler::Scheduler;

#[derive(Default)]
pub struct World<'w> {
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    pub(crate) systems: Systems<'w>,
    pub(crate) scheduler: Scheduler,
    pub(crate) events: Events,
    pub(crate) resources: Resources
}

impl<'w> World<'w> {
    pub fn new() -> World<'w> {
        World::default()
    }

    pub fn spawn<'e, B: SpawnBundle>(&'e self, bundle: B) -> Entity<'e> where 'e: 'w {
        let entity = self.entities.alloc();
        bundle.insert_into(&self.components, entity);

        Entity {
            world: self,
            id: entity,
        }
    }

    #[inline]
    pub fn spawn_empty<'e>(&'e self) -> Entity<'e> where 'e: 'w {
        self.spawn(())
    }

    pub fn add_resource<R: Resource>(self: &Arc<Self>, resource: R) {
        self.resources.insert(resource)
    }

    pub fn add_system<P, S>(&self, system: S)
    where
        P: SystemParams<'w> + Send + Sync + 'static,
        S: ParameterizedSystem<'w, P> + Send + Sync + 'static,
        SystemContainer<'w, P, S>: System<'w>,
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
