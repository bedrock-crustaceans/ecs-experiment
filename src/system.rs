use std::{marker::PhantomData, sync::Arc};

use crate::{
    event::{Event, EventReader, EventWriter},
    query::{FilterBundle, Query},
    resource::{Res, ResMut, Resource},
    Components, EntityIter, QueryBundle, World,
};

pub trait System {
    fn call(&self, world: Arc<World>);
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct SystemObject<Params, F: RawSystem<Params>> {
    pub system: F,
    pub phantom: PhantomData<Params>,
}

impl<F: RawSystem<()>> System for SystemObject<(), F> {
    fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

impl<P0, F: RawSystem<P0>> System for SystemObject<P0, F>
where
    P0: SystemParam,
{
    fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

impl<P0, P1, F: RawSystem<(P0, P1)>> System for SystemObject<(P0, P1), F>
where
    P0: SystemParam,
    P1: SystemParam,
{
    fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

pub trait SystemParam {
    const EXCLUSIVE: bool;

    fn fetch(world: Arc<World>) -> Self;
}

impl<'a, C: QueryBundle, F: FilterBundle> SystemParam for Query<'a, C, F> {
    // const EXCLUSIVE: bool = C::EX;
    const EXCLUSIVE: bool = true;

    fn fetch(world: Arc<World>) -> Self {
        Query::new(world)
    }
}

impl<'a, R: Resource> SystemParam for Res<'a, R> {
    const EXCLUSIVE: bool = false;

    fn fetch(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<'a, R: Resource> SystemParam for ResMut<'a, R> {
    const EXCLUSIVE: bool = true;

    fn fetch(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<E: Event> SystemParam for EventWriter<E> {
    const EXCLUSIVE: bool = false;

    fn fetch(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<E: Event> SystemParam for EventReader<E> {
    const EXCLUSIVE: bool = false;

    fn fetch(world: Arc<World>) -> Self {
        todo!();
    }
}

pub trait RawSystem<Params>: Sized {
    fn into_object(self) -> SystemObject<Params, Self> {
        SystemObject {
            system: self,
            phantom: PhantomData,
        }
    }

    fn call(&self, world: Arc<World>);
}

impl<F> RawSystem<()> for F
where
    F: Fn(),
{
    fn call(&self, _world: Arc<World>) {
        self();
    }
}

impl<F, P0> RawSystem<P0> for F
where
    F: Fn(P0),
    P0: SystemParam,
{
    fn call(&self, world: Arc<World>) {
        let p0 = P0::fetch(world);
        self(p0);
    }
}

impl<F, P0, P1> RawSystem<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam,
    P1: SystemParam,
{
    fn call(&self, world: Arc<World>) {
        let p0 = P0::fetch(world.clone());
        let p1 = P1::fetch(world);
        self(p0, p1);
    }
}
