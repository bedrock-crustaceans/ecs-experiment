use std::marker::PhantomData;

use crate::{Component, Components, Entity, Filter};

pub trait InsertionBundle {
    fn insert_into(self, components: &mut Components, entity: Entity);
}

impl<'a, C: Component> Component for &'a C {}
impl<'a, C: Component> Component for &'a mut C {}

impl<'a, C0: Component + 'static> InsertionBundle for C0 {
    fn insert_into(self, components: &mut Components, entity: Entity) {
        components.insert(entity, self);
    }
}

impl<'a, C0, C1> InsertionBundle for (C0, C1)
where
    C0: Component + 'static,
    C1: Component + 'static,
{
    fn insert_into(self, components: &mut Components, entity: Entity) {
        components.insert(entity, self.0);
        components.insert(entity, self.1);
    }
}

pub trait QueryBundle {
    const MUTABLE: bool;
}

impl<'a, C0: Component> QueryBundle for C0 {
    const MUTABLE: bool = false;
}

impl<'a, C0, C1> QueryBundle for (C0, C1)
where
    C0: Component,
    C1: Component,
{
    const MUTABLE: bool = <&C0>::MUTABLE || <&C1>::MUTABLE;
}

pub trait FilterBundle {}

impl FilterBundle for () {}

impl<F: Filter> FilterBundle for F {}

impl<F0, F1> FilterBundle for (F0, F1)
where
    F0: FilterBundle,
    F1: FilterBundle,
{
}

pub struct Query<C: QueryBundle, F: FilterBundle = ()> {
    collection: C,
    _marker: PhantomData<F>,
}
