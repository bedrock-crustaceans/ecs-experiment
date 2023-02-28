use std::{any::TypeId, marker::PhantomData, sync::Arc};

use crate::{Component, ComponentStorage, Components, Entity, EntityIter, Filter, World};

pub trait InsertionBundle {
    fn insert_into(self, components: &Components, entity: Entity);
}

impl<'a, C: Component> Component for &'a C {}

impl<'a, C: Component> Component for &'a mut C {}

impl<'a, C0: Component + 'static> InsertionBundle for C0 {
    fn insert_into(self, components: &Components, entity: Entity) {
        components.insert(entity, self);
    }
}

impl<'a, C0, C1> InsertionBundle for (C0, C1)
where
    C0: Component + 'static,
    C1: Component + 'static,
{
    fn insert_into(self, components: &Components, entity: Entity) {
        components.insert(entity, self.0);
        components.insert(entity, self.1);
    }
}

pub trait QueryBundle: Sized {
    const MUTABLE: bool;

    fn fetch<'a, F: FilterBundle>(entity: Entity, components: &'a Components) -> Option<Self>;
}

impl<'a, C0: Component> QueryBundle for C0 {
    const MUTABLE: bool = false;

    fn fetch<'b, F: FilterBundle>(entity: Entity, components: &'b Components) -> Option<C0> {
        todo!();

        None
    }
}

impl<'a, C0, C1> QueryBundle for (C0, C1)
where
    C0: Component,
    C1: Component,
{
    const MUTABLE: bool = <&C0>::MUTABLE || <&C1>::MUTABLE;

    fn fetch<'b, F: FilterBundle>(entity: Entity, components: &'b Components) -> Option<(C0, C1)> {
        None
    }
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
    entity_iter: EntityIter,
    world: Arc<World>,
    phantom: PhantomData<(C, F)>,
}

impl<C: QueryBundle, F: FilterBundle> Iterator for Query<C, F> {
    type Item = C;

    fn next(&mut self) -> Option<C> {
        let entity = self.entity_iter.next()?;
        C::fetch::<F>(entity, &self.world.components)
    }
}

impl<C: QueryBundle, F: FilterBundle> Query<C, F> {
    pub fn new(world: Arc<World>) -> Self {
        Self {
            entity_iter: EntityIter::new(world.clone()),
            world,
            phantom: PhantomData,
        }
    }
}
