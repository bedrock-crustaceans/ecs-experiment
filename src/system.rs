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
pub struct GenericSystem<'a, Params, F: RawSystem<'a, Params>> {
    pub system: F,
    pub phantom: PhantomData<Params>,
}

impl<F: RawSystem<'a, ()>> System for GenericSystem<(), F> {
    fn call(&self, world: &World) {
        self.system.call(world);
    }
}

impl<'a, P0, F: RawSystem<'a, P0>> System for GenericSystem<P0, F>
where
    P0: SystemParam<'a>,
{
    fn call(&'a self, world: &World) {
        self.system.call(world);
    }
}

impl<'a, P0, P1, F: RawSystem<'a, (P0, P1)>> System for GenericSystem<(P0, P1), F>
where
    P0: SystemParam<'a>,
    P1: SystemParam<'a>,
{
    fn call(&self, world: &World) {
        self.system.call(world);
    }
}

pub trait SystemParam<'a>: Sized {
    const EXCLUSIVE: bool;

    fn fetch(world: &'a World) -> Self {
        panic!(
            "{} does not support immutable fetching",
            std::any::type_name::<Self>()
        );
    }

    fn fetch_mut(world: &'a mut World) -> Self {
        panic!(
            "{} does not support mutable fetching",
            std::any::type_name::<Self>()
        );
    }
}

impl<'a, C: QueryBundle, F: FilterBundle> SystemParam<'a> for Query<'a, C, F> {
    const EXCLUSIVE: bool = C::MUTABLE;

    fn fetch(world: &'a World) -> Self {
        Query::from(world)
    }
}

impl<'a, R: Resource> SystemParam<'a> for Res<'a, R> {
    const EXCLUSIVE: bool = false;
}

impl<'a, R: Resource> SystemParam<'a> for ResMut<'a, R> {
    const EXCLUSIVE: bool = true;
}

impl<E: Event> SystemParam<'_> for EventWriter<E> {
    const EXCLUSIVE: bool = false;
}

impl<E: Event> SystemParam<'_> for EventReader<E> {
    const EXCLUSIVE: bool = false;
}

pub trait RawSystem<'a, Params>: Sized {
    fn into_generic(self) -> GenericSystem<Params, Self> {
        GenericSystem {
            system: self,
            phantom: PhantomData,
        }
    }

    fn call(&self, world: &'a World);
}

impl<'a, F> RawSystem<'a, ()> for F
where
    F: Fn(),
{
    fn call(&self, _world: &'a World) {
        self();
    }
}

impl<'a, F, P0> RawSystem<'a, P0> for F
where
    F: Fn(P0),
    P0: SystemParam<'a>,
{
    fn call(&self, world: &'a World) {
        let p0 = P0::fetch(world);
        self(p0);
    }
}

impl<'a, F, P0, P1> RawSystem<'a, (P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam<'a>,
    P1: SystemParam<'a>,
{
    fn call(&self, world: &'a World) {
        todo!();
    }
}
