use std::marker::PhantomData;

use crate::{
    event::{Event, EventReader, EventWriter},
    query::{FilterBundle, InsertionBundle, Query},
    resource::{Res, ResMut, Resource},
    QueryBundle, World,
};

pub trait SystemParamBundle: Sized {}

impl<P0> SystemParamBundle for P0 where P0: SystemParam {}

impl<P0, P1> SystemParamBundle for (P0, P1)
where
    P0: SystemParam,
    P1: SystemParam,
{
}

pub trait System {
    fn call(&self, world: &World);
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct GenericSystem<Params: SystemParamBundle, F: IntoSystem<Params>> {
    callable: F,
    _marker: PhantomData<Params>,
}

impl<P0, F: Fn(P0)> System for GenericSystem<P0, F>
where
    P0: SystemParam,
{
    fn call(&self, world: &World) {
        // println!("1 parameter");
        // dbg!(std::any::type_name::<P0>());
        // (self.callable)(P0::default());
    }
}

impl<P0, P1, F: Fn(P0, P1)> System for GenericSystem<(P0, P1), F>
where
    P0: SystemParam,
    P1: SystemParam,
{
    fn call(&self, world: &World) {
        // println!("2 parameters");
        // dbg!(std::any::type_name::<(P0, P1)>());
        // (self.callable)(P0::default(), P1::default());
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

pub trait IntoSystem<Params: SystemParamBundle>: Sized {
    fn into_system(self) -> GenericSystem<Params, Self> {
        GenericSystem {
            callable: self,
            _marker: PhantomData,
        }
    }
}

impl<F, P0: SystemParam> IntoSystem<P0> for F where F: Fn(P0) {}
impl<F, P0, P1> IntoSystem<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam,
    P1: SystemParam,
{
}
