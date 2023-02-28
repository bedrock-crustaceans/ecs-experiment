use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::Arc,
};

use crate::{
    Component, ComponentStorage, Components, Entity, EntityIter, Filter, StorageFetch, World,
};

trait TyEq {}

impl<T> TyEq for (T, T) {}

pub trait InsertionBundle {
    fn insert_into(self, components: &Components, entity: Entity);
}

pub trait ComponentRef: Sized {
    type NonRef: Component;

    const SHARED: bool;
}

impl<'a, C: Component> ComponentRef for &'a C {
    type NonRef = C;

    const SHARED: bool = true;
}

impl<'a, C: Component> ComponentRef for &'a mut C {
    type NonRef = C;

    const SHARED: bool = false;
}

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
    // const SHARED: bool;

    type Output;

    fn fetch<F: FilterBundle>(entity: Entity, components: &Components) -> Option<Self::Output>;
}

impl<'a, C1: Component> QueryBundle for &'a C1 {
    // const SHARED: bool = C1::SHARED;

    type Output = &'a C1;

    fn fetch<F: FilterBundle>(entity: Entity, components: &Components) -> Option<&'a C1> {
        let kv = components.storage.get(&C1::id())?;
        kv.value().as_ref().fetch::<C1>()
        // todo!();
    }
}

impl<'a, C0, C1> QueryBundle for (C0, C1)
where
    C0: QueryBundle,
    C1: QueryBundle,
{
    // const SHARED: bool = C0::SHARED || C1::SHARED;

    type Output = (C0, C1);

    fn fetch<'b, F: FilterBundle>(entity: Entity, components: &'b Components) -> Option<Self> {
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
    type Item = C::Output;

    fn next(&mut self) -> Option<C::Output> {
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
