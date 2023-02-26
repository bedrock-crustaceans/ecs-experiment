use std::marker::PhantomData;

use crate::{
    event::{Event, EventReader, EventWriter},
    query::{ComponentCollection, FilterCollection, Query},
    resource::{Res, ResMut, Resource},
};

pub trait System {
    fn print(&self);
}

pub trait SystemParamCollection {
    fn print();
}

impl<P> SystemParamCollection for P where P: SystemParam {
    fn print() {
        println!("This collection has 1 parameter");
        dbg!(std::any::type_name::<P>());
    }
}

impl<P0, P1> SystemParamCollection for (P0, P1)
where
    P0: SystemParam, P1: SystemParam
{
    fn print() {
        println!("This collection has 2 parameters");
        dbg!(std::any::type_name::<P0>());
        dbg!(std::any::type_name::<P1>());
    }
}

pub struct BoxedSystem<Params: SystemParamCollection> {
    callable: Box<dyn IntoSystem<Params>>
}

impl<Params: SystemParamCollection> System for BoxedSystem<Params> {
    fn print(&self) {
        Params::print();
    }
}

pub trait SystemParam {
    const EXCLUSIVE: bool;
}

impl<T: ComponentCollection, F: FilterCollection> SystemParam for Query<T, F> {
    const EXCLUSIVE: bool = T::MUTABLE;
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

pub trait IntoSystem<P: SystemParamCollection> {
    fn into_descriptor(self) -> SystemDescriptor<P>;
}

impl<F, P: SystemParam> IntoSystem<P> for F
where
    F: Fn(P) + 'static,
{
    fn into_descriptor(self) -> SystemDescriptor<P> {
        P::print();

        SystemDescriptor {
            system: BoxedSystem {
                callable: Box::new(self)
            },
            exclusive: P::EXCLUSIVE
        }
    }
}

impl<F, P0, P1> IntoSystem<(P0, P1)> for F
where
    F: Fn(P0, P1) + 'static,
    P0: SystemParam,
    P1: SystemParam,
{
    fn into_descriptor(self) -> SystemDescriptor<(P0, P1)> {
        <(P0, P1)>::print();
        // P1::print();

        SystemDescriptor {
            system: BoxedSystem {
                callable: Box::new(self)
            },
            exclusive: P0::EXCLUSIVE || P1::EXCLUSIVE
        }
    }
}

pub struct SystemDescriptor<Params: SystemParamCollection> {
    system: BoxedSystem<Params>,
    exclusive: bool
}
