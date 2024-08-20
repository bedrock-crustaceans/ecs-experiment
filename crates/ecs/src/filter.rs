use crate::{Component, Entity};
use std::marker::PhantomData;

pub trait Filter {
    fn filter(entity: &Entity) -> bool;
}

pub struct With<T: Component> {
    _marker: PhantomData<T>,
}

impl<T: Component> Filter for With<T> {
    fn filter(entity: &Entity) -> bool {
        entity.has::<T>()
    }
}

pub struct Without<T: Component> {
    _marker: PhantomData<T>,
}

impl<T: Component> Filter for Without<T> {
    fn filter(entity: &Entity) -> bool {
        !entity.has::<T>()
    }
}

pub struct Added<T: Component> {
    _marker: PhantomData<T>,
}

impl<T: Component> Filter for Added<T> {
    fn filter(_entity: &Entity) -> bool {
        todo!()
    }
}

pub struct Removed<T: Component> {
    _marker: PhantomData<T>,
}

impl<T: Component> Filter for Removed<T> {
    fn filter(_entity: &Entity) -> bool {
        todo!()
    }
}

pub struct Changed<T: Component> {
    _marker: PhantomData<T>,
}

impl<T: Component> Filter for Changed<T> {
    fn filter(_entity: &Entity) -> bool {
        todo!()
    }
}

pub trait FilterParams {
    fn filter(entity: &Entity) -> bool;
}

impl FilterParams for () {
    fn filter(_entity: &Entity) -> bool {
        true
    }
}

impl<F: Filter> FilterParams for F {
    fn filter(entity: &Entity) -> bool {
        F::filter(&entity)
    }
}

impl<F0, F1> FilterParams for (F0, F1)
where
    F0: FilterParams,
    F1: FilterParams,
{
    fn filter(entity: &Entity) -> bool {
        F0::filter(entity) && F1::filter(entity)
    }
}
