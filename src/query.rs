use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::Arc, collections::HashMap,
};

use dashmap::mapref::one::Ref;
use parking_lot::RwLockReadGuard;

use crate::{
    Component, ComponentStorage, Components, Entity, EntityIter, Filter, World, Storage, GenericStorage,
};

trait TyEq {}

impl<T> TyEq for (T, T) {}

pub trait InsertionBundle {
    fn insert_into(self, components: &Components, entity: Entity);
}

// pub trait ComponentRef: Sized {
//     type NonRef: Component;

//     const SHARED: bool;
// }

// impl<'a, C: Component> ComponentRef for &'a C {
//     type NonRef = C;

//     const SHARED: bool = true;
// }

// impl<'a, C: Component> ComponentRef for &'a mut C {
//     type NonRef = C;

//     const SHARED: bool = false;
// }

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

    type NonRef;

    // type Output<'b> where 'b: 'a;

    // fn fetch<F: FilterBundle>(
    //     entity: Entity, map: &'a HashMap<TypeId, Arc<dyn Storage + Send + Sync>>
    // ) -> Option<Self::Output<'a>>;
}

impl<C1: Component> QueryBundle for &C1 {
    // const SHARED: bool = C1::SHARED;

    type NonRef = C1;

    // type Output<'b> = &'a C1 where 'b: 'a;

    // fn fetch<F: FilterBundle>(
    //     entity: Entity, map: &'a HashMap<TypeId, Arc<dyn Storage + Send + Sync>>
    // ) -> Option<Self::Output<'a>> {
    //     // map.get(&C1::id())?.as_ref().fetch::<C1>()
    //     todo!();
    // }
}

impl<C0, C1> QueryBundle for (C0, C1)
where
    C0: QueryBundle,
    C1: QueryBundle,
{
    type NonRef = (C0::NonRef, C1::NonRef);

    // const SHARED: bool = C0::SHARED || C1::SHARED;

    // type Output<'b> = (C0, C1) where 'b: 'a;

    // fn fetch<'b, F: FilterBundle>(entity: Entity, map: &'a HashMap<TypeId, Arc<dyn Storage + Send + Sync>>) -> Option<Self> {
    //     todo!();
    // }
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

pub struct Query<'w, C: QueryBundle, F: FilterBundle = ()> {
    entity_iter: EntityIter,
    world: Arc<World>,
    phantom: PhantomData<&'w (C, F)>,
}

impl<'a, C: Component + 'a, B, F: FilterBundle> Iterator for Query<'a, B, F> 
    where B: QueryBundle<NonRef = C>
{
    type Item = &'a B::NonRef;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let lock = self.world.components.storage.read();
        let storage = lock.get(&C::id())?;

        let entity = self.entity_iter.next()?;
        Some(unsafe {
            &*(storage.fetch(entity)? as *const C)
        })

        // todo!();
        // C::fetch::<F>(entity, &*self.world.as_ref().components.storage.read())
    }
}

impl<'a, C: QueryBundle, F: FilterBundle> Query<'a, C, F> {
    pub fn new(world: Arc<World>) -> Self {
        Self {
            entity_iter: EntityIter::new(world.clone()),
            world,
            phantom: PhantomData,
        }
    }
}
