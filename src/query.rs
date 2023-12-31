use std::{
    marker::PhantomData,
    sync::Arc
};
use std::any::TypeId;

use crate::{
    Component, Components, EntityId, Filter, World
};
use crate::world::TypedStorage;

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

    unsafe fn unlock_all(components: &Components);
}

impl<T: Component + 'static> QueryBundle for &T {
    type NonRef = T;
    const SHARED: bool = true;

    unsafe fn unlock_all(components: &Components) {
        let typeless_store = components.map.get(&TypeId::of::<T>());
        if let Some(store) = typeless_store {
            let typed_store = store
                .value()
                .as_any()
                .downcast_ref::<TypedStorage<T>>()
                .unwrap();

            // Release the component lock
            unsafe {
                typed_store.storage.force_unlock_read();
            }

            dbg!("Unlocked read lock");
        }
    }
}

impl<T: Component> QueryBundle for &mut T {
    type NonRef = T;
    const SHARED: bool = false;

    unsafe fn unlock_all(components: &Components) { todo!() }
}

impl<T1, T2> QueryBundle for (T1, T2)
where
    T1: QueryBundle,
    T2: QueryBundle,
{
    type NonRef = (T1::NonRef, T2::NonRef);
    const SHARED: bool = T1::SHARED || T2::SHARED;

    unsafe fn unlock_all(components: &Components) { todo!() }
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

pub struct Query<Q: QueryBundle, F: FilterBundle = ()> {
    world: Arc<World>,
    _marker: PhantomData<(Q, F)>,
}

impl<'query, C, F, N> IntoIterator for &'query Query<N, F>
where
    C: Component + 'static, F: FilterBundle, N: QueryBundle<NonRef = C>
{
    type Item =  &'query N::NonRef;
    type IntoIter = QueryIter<'query, C, N, F>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter {
            world: self.world.clone(),
            index: 0,
            _marker: PhantomData
        }
    }
}

pub struct QueryIter<'query, C, Q, F>
where
    Q: QueryBundle<NonRef = C>,
    F: FilterBundle
{
    world: Arc<World>,
    index: usize,
    _marker: PhantomData<&'query (Q, F)>
}

impl<'query, C, F, N> Iterator for QueryIter<'query, C, N, F>
where
    C: Component + 'static, F: FilterBundle, N: QueryBundle<NonRef = C>
{
    type Item = &'query N::NonRef;

    fn next(&mut self) -> Option<Self::Item> {
        let typeless_store = self.world.components.map
            .get(&TypeId::of::<N::NonRef>());

        if let Some(store) = typeless_store {
            let typed_store = store
                .value()
                .as_any()
                .downcast_ref::<TypedStorage<N::NonRef>>()
                .unwrap();

            // Lock the store while QueryIter exists.
            let store_lock = if self.index == 0 {
                // Acquire lock
                typed_store.storage.read()
            } else {
                // Retrieve lock
                unsafe {
                    typed_store.storage.make_read_guard_unchecked()
                }
            };

            let store_index = typed_store.reverse_map.read().get(self.index).map(|id| *id);
            if store_index.is_none() {
                // No more components remaining.
                std::mem::forget(store_lock);
                return None
            };

            // let store_index = store_index.unwrap();
            let item = match N::SHARED {
                true => {
                    Some(unsafe {
                        &*(&store_lock[self.index] as *const N::NonRef)
                    })
                    // Some(&store_lock[self.index])
                },
                false => {
                    todo!("Single mutable fetch")
                }
            };

            std::mem::forget(store_lock);
            self.index += 1;
            item
        } else {
            dbg!("yes, none");
            // This component is not owned by any entity
            None
        }
    }
}

impl<'query, C, Q, F> Drop for QueryIter<'query, C, Q, F>
where
    Q: QueryBundle<NonRef = C>,
    F: FilterBundle
{
    fn drop(&mut self) {
        unsafe { Q::unlock_all(&self.world.components); }
    }
}

impl<C: QueryBundle, F: FilterBundle> Query<C, F> {
    pub fn new(world: Arc<World>) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }
}
