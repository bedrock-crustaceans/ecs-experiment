use std::marker::PhantomData;

use crate::{
    event::{Event, EventReader, EventWriter},
    query::{FilterBundle, Query},
    resource::{Res, ResMut, Resource},
    QueryBundle, World,
};

pub trait System {
    fn call(&self, world: &World);
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct GenericSystem<Params, F: RawSystem<Params>> {
    pub system: F,
    pub phantom: PhantomData<Params>,
}

impl<F: RawSystem<()>> System for GenericSystem<(), F> {
    fn call(&self, world: &World) {
        self.system.call(world);
    }
}

impl<P0, F: RawSystem<P0>> System for GenericSystem<P0, F>
where
    P0: SystemParam,
{
    fn call(&self, world: &World) {
        self.system.call(world);
    }
}

impl<P0, P1, F: RawSystem<(P0, P1)>> System for GenericSystem<(P0, P1), F>
where
    P0: SystemParam,
    P1: SystemParam,
{
    fn call(&self, world: &World) {
        self.system.call(world);
    }
}

pub trait SystemParam {
    const EXCLUSIVE: bool;
}

impl<C: QueryBundle, F: FilterBundle> SystemParam for Query<C, F> {
    const EXCLUSIVE: bool = C::MUTABLE;
}

impl<'a, R: Resource> SystemParam for Res<'a, R> {
    const EXCLUSIVE: bool = false;
}

impl<'a, R: Resource> SystemParam for ResMut<'a, R> {
    const EXCLUSIVE: bool = true;
}

impl<E: Event> SystemParam for EventWriter<E> {
    const EXCLUSIVE: bool = false;
}

impl<E: Event> SystemParam for EventReader<E> {
    const EXCLUSIVE: bool = false;
}

pub trait RawSystem<Params>: Sized {
    fn into_generic(self) -> GenericSystem<Params, Self> {
        GenericSystem {
            system: self,
            phantom: PhantomData,
        }
    }

    fn call(&self, world: &World);
}

impl<F> RawSystem<()> for F
where
    F: Fn(),
{
    fn call(&self, _world: &World) {
        self();
    }
}

impl<F, P0> RawSystem<P0> for F
where
    F: Fn(P0),
    P0: SystemParam,
{
    fn call(&self, world: &World) {
        todo!();
    }
}

impl<F, P0, P1> RawSystem<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam,
    P1: SystemParam,
{
    fn call(&self, world: &World) {
        todo!();
    }
}
