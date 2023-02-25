use crate::{
    event::{Event, EventReader, EventWriter},
    query::{ComponentCollection, FilterCollection, Query},
    resource::{Res, ResMut, Resource},
};

pub trait SystemParam {
    const EXCLUSIVE: bool;

    fn print();
}

impl<T: ComponentCollection, F: FilterCollection> SystemParam for Query<T, F> {
    const EXCLUSIVE: bool = T::MUTABLE;

    fn print() {
        println!(
            "{}\n{}",
            std::any::type_name::<T>(),
            std::any::type_name::<F>()
        );
    }
}

impl<'a, R: Resource> SystemParam for Res<'a, R> {
    const EXCLUSIVE: bool = false;

    fn print() {
        println!("{}", std::any::type_name::<R>());
    }
}

impl<'a, R: Resource> SystemParam for ResMut<'a, R> {
    const EXCLUSIVE: bool = true;

    fn print() {
        println!("{}", std::any::type_name::<R>());
    }
}

impl<E: Event> SystemParam for EventWriter<E> {
    const EXCLUSIVE: bool = false;

    fn print() {
        println!("{}", std::any::type_name::<E>());
    }
}

impl<E: Event> SystemParam for EventReader<E> {
    const EXCLUSIVE: bool = false;

    fn print() {
        println!("{}", std::any::type_name::<E>());
    }
}

pub trait IntoSystemDescriptor<P> {
    fn into_descriptor(&self) -> SystemDescriptor;
}

impl<F, P: SystemParam> IntoSystemDescriptor<P> for F
where
    F: Fn(P),
{
    fn into_descriptor(&self) -> SystemDescriptor {
        P::print();

        SystemDescriptor {
            exclusive: P::EXCLUSIVE,
        }
    }
}

impl<F, P0, P1> IntoSystemDescriptor<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam,
    P1: SystemParam,
{
    fn into_descriptor(&self) -> SystemDescriptor {
        P0::print();
        P1::print();

        SystemDescriptor {
            exclusive: P0::EXCLUSIVE || P1::EXCLUSIVE,
        }
    }
}

#[derive(Debug)]
pub struct SystemDescriptor {
    exclusive: bool,
}
