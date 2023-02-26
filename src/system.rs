use std::marker::PhantomData;

use crate::{
    event::{Event, EventReader, EventWriter},
    query::{ComponentBundle, FilterBundle, Query},
    resource::{Res, ResMut, Resource},
};

pub trait System {
    fn print(&self);
}

pub trait SystemParamBundle {
    fn print();
}

impl<P> SystemParamBundle for P
where
    P: SystemParam,
{
    fn print() {
        println!("This collection has 1 parameter");
        dbg!(std::any::type_name::<P>());
    }
}

impl<P0, P1> SystemParamBundle for (P0, P1)
where
    P0: SystemParam,
    P1: SystemParam,
{
    fn print() {
        println!("This collection has 2 parameters");
        dbg!(std::any::type_name::<P0>());
        dbg!(std::any::type_name::<P1>());
    }
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct GenericSystem<Params: SystemParamBundle, F: IntoSystem<Params>> {
    callable: F,
    _marker: PhantomData<Params>,
}

impl<Params: SystemParamBundle, F: IntoSystem<Params>> System for GenericSystem<Params, F> {
    fn print(&self) {
        Params::print();
    }
}

pub trait SystemParam {
    const EXCLUSIVE: bool;
}

impl<C: ComponentBundle, F: FilterBundle> SystemParam for Query<C, F> {
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
    // fn into_descriptor(self) -> SystemDescriptor<P, Self>;
    fn into_system(self) -> GenericSystem<Params, Self> {
        GenericSystem {
            callable: self,
            _marker: PhantomData,
        }
    }
}

impl<F, P: SystemParam> IntoSystem<P> for F
where
    F: Fn(P),
{
    // fn into_descriptor(self) -> SystemDescriptor<P, F> {
    //     SystemDescriptor {
    //         system: GenericSystem {
    //             callable: self,
    //             _marker: PhantomData,
    //         },
    //         exclusive: P::EXCLUSIVE,
    //     }
    // }
}

impl<F, P0, P1> IntoSystem<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam,
    P1: SystemParam,
{
    // fn into_descriptor(self) -> SystemDescriptor<(P0, P1), F> {
    //     SystemDescriptor {
    //         system: GenericSystem {
    //             callable: self,
    //             _marker: PhantomData,
    //         },
    //         exclusive: P0::EXCLUSIVE || P1::EXCLUSIVE,
    //     }
    // }
}

// pub struct SystemDescriptor<Params: SystemParamBundle, F: IntoSystem<Params>> {
//     pub system: GenericSystem<Params, F>,
//     exclusive: bool,
// }
