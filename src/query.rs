use std::{
    marker::PhantomData,
    sync::Arc
};

use crate::{
    Component, Components, EntityId, Filter, World
};

trait TyEq {}

impl<T> TyEq for (T, T) {}

pub trait SpawnBundle {
    fn insert_into(self, components: &Components, entity: EntityId);
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

impl SpawnBundle for () {
    fn insert_into(self, _components: &Components, _entity: EntityId) {}
}

impl<'a, C0: Component + 'static> SpawnBundle for C0 {
    fn insert_into(self, components: &Components, entity: EntityId) {
        components.insert(entity, self);
    }
}

impl<'a, C0, C1> SpawnBundle for (C0, C1)
where
    C0: Component + 'static,
    C1: Component + 'static,
{
    fn insert_into(self, components: &Components, entity: EntityId) {
        components.insert(entity, self.0);
        components.insert(entity, self.1);
    }
}

pub trait QueryBundle: Sized {
    type NonRef;
    const SHARED: bool;
}

impl<T: Component> QueryBundle for &T {
    type NonRef = T;
    const SHARED: bool = true;
}

impl<T: Component> QueryBundle for &mut T {
    type NonRef = T;
    const SHARED: bool = false;
}

impl<T1, T2> QueryBundle for (T1, T2)
where
    T1: QueryBundle,
    T2: QueryBundle,
{
    type NonRef = (T1::NonRef, T2::NonRef);
    const SHARED: bool = T1::SHARED || T2::SHARED;
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

pub struct Query<'w, Q: QueryBundle, F: FilterBundle = ()> {
    world: Arc<World>,
    _marker: PhantomData<&'w (Q, F)>,
}

impl<'w, C, F, N> IntoIterator for Query<'w, N, F>
where
    C: Component + 'w, F: FilterBundle, N: QueryBundle<NonRef = C>
{
    type Item =  &'w N::NonRef;
    type IntoIter = QueryIter<'w, N, F>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter {
            world: self.world,
            index: 0,
            _marker: PhantomData
        }
    }
}

pub struct QueryIter<'w, Q: QueryBundle, F: FilterBundle> {
    world: Arc<World>,
    index: usize,
    _marker: PhantomData<&'w (Q, F)>
}

impl<'w, C, F, N> Iterator for QueryIter<'w, N, F>
where
    C: Component + 'w, F: FilterBundle, N: QueryBundle<NonRef = C>
{
    type Item = &'w N::NonRef;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

// impl<'a, C: Component + 'a, N, F: FilterBundle> Iterator for Query<'a, N, F>
//     where N: QueryBundle<NonRef = C>
// {
//     type Item = &'a N::NonRef;
//
//     #[inline]
//     fn next(&mut self) -> Option<Self::Item> {
//         dbg!(std::any::type_name::<N>());
//
//         // let lock = self.world.components.map.read();
//         // let storage = lock.get(&C::id())?;
//
//         // Some(unsafe {
//         //     &*(storage.fetch(entity)? as *const C)
//         // })
//
//         todo!();
//         // C::fetch::<F>(entity, &*self.world.as_ref().components.map.read())
//     }
// }

impl<'a, C: QueryBundle, F: FilterBundle> Query<'a, C, F> {
    pub fn new(world: Arc<World>) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }
}
