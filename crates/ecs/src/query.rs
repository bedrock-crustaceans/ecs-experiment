// use std::any::TypeId;
// use std::ops::{Deref, DerefMut};
// use std::sync::atomic::{AtomicBool, Ordering};
// use std::{marker::PhantomData, sync::Arc};
// use std::mem::MaybeUninit;

// use crate::component::{Component, Components, TypedStorage};
// use crate::{sealed, Entity, EntityId, EntityIter, Filter, World};
// use crate::filter::FilterParams;
// use crate::sealed::Sealed;

// pub struct QueryGuard<T: Component> {
//     inner: T,
//     world: Arc<World>,
//     _marker: PhantomData<*mut ()>
// }

// impl<T: Component> Deref for QueryGuard<T> {
//     type Target = T;

//     fn deref(&self) -> &T {
//         &self.inner
//     }
// }

// impl<T: Component> DerefMut for QueryGuard<T> {
//     fn deref_mut(&mut self) -> &mut T {
//         &mut self.inner
//     }
// }

// impl<T: Component> Drop for QueryGuard<T> {
//     fn drop(&mut self) {
//         unsafe {
//             T::unlock_one::<sealed::Sealer>(&self.world.components);
//         }
//     }
// }

// pub trait Queryable {
//     type Ref<'q>;
//     type Guard;
// }

// impl Queryable for Entity {
//     type Ref<'q> = Self;
//     type Guard = Entity;
// }

// impl<T: Component> Queryable for T {
//     type Ref<'q> = &'q T;
//     type Guard = QueryGuard<T>;
// }

// impl<T1: Component, T2: Component> Queryable for (T1, T2) {
//     type Ref<'q> = (&'q T1, &'q T2);
//     type Guard = (QueryGuard<T1>, QueryGuard<T2>);
// }

// /// Represents a collection of items contained in a [`Query`].
// ///
// /// Any type that implements this trait can be used in a query.
// pub trait QueryParams: Sized {
//     /// Represents the type that this trait is implemented for, but with all references removed.
//     /// For example: `&Component` becomes `Component`.
//     type Guarded;
//     /// Whether the query contains only shared references.
//     const SHARED: bool;
//     /// Whether this type is [`Entity`].
//     const IS_ENTITY: bool = false;

//     /// Unlocks all used component storages.
//     ///
//     /// The generic parameter exists to prevent consumers of the crate from calling the function.
//     /// This is not done with a private supertrait since that causes all kinds of generic parameter issues
//     /// with [`QueryParams`].
//     ///
//     /// # Safety
//     ///
//     /// This function should only be called in the Drop implementation of [`Query`] when
//     /// the `locked` flag is set to true.
//     /// Calling this function in any other situation will lead to undefined behaviour.
//     #[doc(hidden)]
//     unsafe fn unlock_one<S: sealed::Sealed>(components: &Components);
//     /// Fetches the requested components from a world.
//     /// 
//     /// The `F` generic parameter is there to allow passing filters to the fetch method.
//     /// 
//     /// # Returns
//     /// 
//     /// This function returns a tuple of the fetched component(s) and the amount the iterator index should be advanced by.
//     fn fetch<F: FilterParams>(world: &Arc<World>, index: usize, lock: &LockFlag) -> (Option<Self::Guarded>, usize);
// }

// impl QueryParams for Entity {
//     type Guarded = Entity;

//     const SHARED: bool = true;
//     const IS_ENTITY: bool = true;

//     /// Requesting an entity uses no locking, so this is a no-op.
//     unsafe fn unlock_one<S: Sealed>(_components: &Components) {}

//     fn fetch<F: FilterParams>(world: &Arc<World>, index: usize, lock: &LockFlag) -> (Option<Entity>, usize) {
//         todo!()
//     }
// }

// impl<T: Component + 'static> QueryParams for &T {
//     type Guarded = QueryGuard<T>;

//     const SHARED: bool = true;

//     unsafe fn unlock_one<S: sealed::Sealed>(components: &Components) {
//         let typeless_store = components.map.get(&TypeId::of::<T>());
//         if let Some(store) = typeless_store {
//             let typed_store = store
//                 .value()
//                 .as_any()
//                 .downcast_ref::<TypedStorage<T>>()
//                 .unwrap();

//             let locked = typed_store.storage.is_locked();

//             // Release the component lock
//             // SAFETY: Because of the required guarantees made by the caller.
//             // Unlocking the read lock specifically is valid because this function is only implemented
//             // for shared references, which only utilise shared locks.
//             typed_store.storage.force_unlock_read();
//         }
//     }

//     fn fetch<F: FilterParams>(world: &Arc<World>, mut index: usize, lock: &LockFlag) -> (Option<QueryGuard<T>>, usize) {
//         let start_index = index;

//         let type_id = TypeId::of::<Self::Guarded>();
//         let typeless_store = world.components.map.get(&type_id);
//             if let Some(store) = typeless_store {
//                 let typed_store = store
//                     .value()
//                     .as_any()
//                     .downcast_ref::<TypedStorage<T>>()
//                     .unwrap();

//                 // // Lock the store while QueryIter exists.
//                 // let store_lock = if !lock.is_flagged() {
//                 //     // Acquire lock
//                 //     lock.flag();
//                 //     typed_store.storage.read()
//                 // } else {
//                 //     // Retrieve lock
//                 //     // SAFETY: This is safe because this query has acquired the lock on the first iteration.
//                 //     // Additionally, this lock has been forgotten and therefore this thread logically still owns the lock.
//                 //     // In case a second iterator is created from the same query, the lock flag will still be set.
//                 //     unsafe { typed_store.storage.make_read_guard_unchecked() }
//                 // };

//                 let store_lock = typed_store.storage.read();

//                 // Find next entity ID. This also filters out the entities that do not match the filter.
//                 let store_index = loop {
//                     let Some(store_index) = typed_store.reverse_map.read().get(index).map(|id| *id) else {
//                         // No more components remaining.
//                         std::mem::forget(store_lock);
//                         return (None, index - start_index);
//                     };

//                     let entity = Entity {
//                         id: store_index,
//                         world: world.clone()
//                     };

//                     if F::filter(&entity) {
//                         // Matching entity has been found.
//                         break store_index
//                     }

//                     // Continue to next option
//                     index += 1;
//                 };

//                 // ZSTs need different treatment
//                 let fetched = if std::mem::size_of::<T>() == 0 {
//                     // SAFETY: This is safe because a ZST does not need initialisation.
//                     // It is also impossible to construct unconstructable types such as empty
//                     // enums since the types have to be constructed to add them to an entity
//                     // in the first place.
//                     let zst = unsafe {
//                         MaybeUninit::uninit().assume_init()
//                     };

//                     // Creates a static reference to the ZST. As `T` is zero-sized, `Box::new` does not actually allocate.
//                     Some(Box::leak(Box::new(zst)) as &T)
//                 } else {
//                     // println!("{}", std::any::type_name::<P>());
//                     // println!("{}", std::any::type_name::<Self::Item>());

//                     // SAFETY: This is simply to get around issues with the type system.
//                     // An assertion at the start of the iterator ensures that both types below are
//                     // equal to each other. The transmuted component will also not have a longer
//                     // lifetime than the original because its lifetime will be bounded to the query.
//                     // The container of the component lives longer than the query and it can also not
//                     // be modified while this query exists.
//                     Some(unsafe {
//                         // std::mem::transmute_copy::<&P, Self::Item>(&&store_lock[self.index])
//                         std::mem::transmute_copy::<&T, _>(&&store_lock[store_index.0])
//                         // &store_lock[store_index.0]
//                     })
//                 };

//                 std::mem::forget(store_lock);
//                 index += 1;
                
//                 let fetched = fetched.map(|x| {
//                     QueryGuard {
//                         inner: x,
//                         world: world.clone(),
//                         _marker: PhantomData
//                     }
//                 });

//                 (fetched, index - start_index)
//             } else {
//                 // This component is not owned by any entity
//                 (None, index - start_index)
//             }
//     }
// }

// impl<T: Component> QueryParams for &mut T {
//     type Guarded = QueryGuard<&mut T>;

//     const SHARED: bool = false;

//     unsafe fn unlock_one<S: sealed::Sealed>(components: &Components) {
//         println!("Unlock write");

//         let typeless_store = components.map.get(&TypeId::of::<T>());
//         if let Some(store) = typeless_store {
//             let typed_store = store
//                 .value()
//                 .as_any()
//                 .downcast_ref::<TypedStorage<T>>()
//                 .unwrap();

//             // Release the component lock
//             // SAFETY: Because of the required guarantees made by the caller.
//             // Unlocking the write lock specifically is valid because this function is only implemented
//             // for mutable references, which only utilise write locks.
//             typed_store.storage.force_unlock_write();
//         }
//     }

//     fn fetch<F: FilterParams>(world: &Arc<World>, index: usize, lock: &LockFlag) -> (Option<QueryGuard<T>>, usize) {
//         todo!()
//     }
// }

// impl<T1, T2> QueryParams for (T1, T2)
// where
//     T1: QueryParams,
//     T2: QueryParams,
// {
//     type Guarded = (T1::Guarded, T2::Guarded);

//     const SHARED: bool = T1::SHARED || T2::SHARED;

//     unsafe fn unlock_one<S: sealed::Sealed>(components: &Components) {
//         T1::unlock_one::<sealed::Sealer>(components);
//         T2::unlock_one::<sealed::Sealer>(components);
//     }

//     fn fetch<F: FilterParams>(world: &Arc<World>, mut index: usize, lock: &LockFlag) 
//         -> (Option<(QueryGuard<T1>, QueryGuard<T2>)>, usize) 
//     {
//         println!("Fetching {} and {}", std::any::type_name::<T1>(), std::any::type_name::<T2>());

//         let start = index;

//         let (t1, adv) = T1::fetch::<F>(world, index, lock);
//         index += adv;

//         let Some(t1) = t1 else {
//             return (None, index - start)
//         };

//         println!("Fetched T1");

//         let (t2, adv) = T2::fetch::<F>(world, index, lock);
//         index += adv;

//         let Some(t2) = t2 else {
//             return (None, index - start)
//         };

//         println!("Fetched T2");

//         // (Some((t1, t2)), index - start)
//         todo!()
//     }
// }

// /// For safety reasons, the locked flag in a query should never be set to false once it has been
// /// set to true. This type enforces that rule by not providing any methods to set it to false.
// #[derive(Default)]
// struct LockFlag {
//     flag: AtomicBool,
// }

// impl LockFlag {
//     /// Signals the flag.
//     #[inline]
//     pub fn flag(&self) {
//         self.flag.store(true, Ordering::SeqCst);
//     }

//     /// Returns whether this flag has been flagged.
//     #[inline]
//     pub fn is_flagged(&self) -> bool {
//         self.flag.load(Ordering::SeqCst)
//     }
// }

// /// A query consists of resources and filters. The first generic parameter of the query contains
// /// the resources. This can for example be a type implementing [`Component`]
// /// or a [`Res`](crate::resource::Res).
// /// Additionally, tuples of mixed types are also allowed to request multiple resources at once.
// ///
// /// # Example
// /// ```rust
// /// # use ecs::{Component, World, Query};
// /// # struct Health { value: f32 }
// /// # impl Component for Health {}
// /// #
// /// # #[tokio::main]
// /// # async fn main() {
// /// #   let world = World::new();
// /// #   world.system(health_display);
// /// #   world.spawn(Health { value: 1.0 });
// /// #   world.tick().await;
// /// # }
// /// fn health_display(query: Query<&Health>) {
// ///     for health in &query {
// ///         println!("Entity has {} health points", health.value);
// ///     }
// /// }
// /// ```
// /// This example requests the `Health` component and a reference to every entity that has the
// /// `Alive` component.
// ///
// /// # Concurrency
// /// The query is created in an unlocked state and can be turned into an iterator.
// /// Once the iterator performs its first iteration, the query turns into a locked query.
// /// This means that all component storages that the query requires will be locked.
// /// Whether this constitutes a shared lock or exclusive lock depends on the content of the query.
// pub struct Query<Q: QueryParams, F: FilterParams = ()> {
//     /// Pointer to the world that this query is directed at.
//     world: Arc<World>,
//     /// Whether this query has acquired locks on component storages.
//     locked: LockFlag,
//     /// Suppresses the unused parameter errors.
//     _marker: PhantomData<(Q, F)>,
// }

// impl<Q: QueryParams, F: FilterParams> Query<Q, F> {
//     /// Creates a new unlocked query for the specified world.
//     pub(crate) fn new(world: Arc<World>) -> Self {
//         Self {
//             world,
//             locked: LockFlag::default(),
//             _marker: PhantomData,
//         }
//     }
// }

// impl<'query, P, F, N> IntoIterator for &'query Query<N, F>
// where
//     P: Queryable + 'static,
//     F: FilterParams,
//     N: QueryParams<Guarded = P>,
// {
//     // type Item = P::Ref<'query>;
//     type Item = P::Guard;
//     type IntoIter = QueryIter<'query, P, N, F>;

//     fn into_iter(self) -> Self::IntoIter {
//         QueryIter {
//             world: self.world.clone(),
//             entity_iter: self.world.entities.iter(self.world.clone()),
//             locked: &self.locked,
//             index: 0,
//             _marker: PhantomData,
//         }
//     }
// }

// impl<Q, F> Drop for Query<Q, F>
// where
//     Q: QueryParams,
//     F: FilterParams,
// {
//     fn drop(&mut self) {
//         if self.locked.is_flagged() {
//             // SAFETY: This is safe because the lock flag has been flagged.
//             // This flag means that the store of every component type is currently locked.
//             // It is therefore to unlock all of the stores.
//             unsafe { Q::unlock_one::<sealed::Sealer>(&self.world.components) }
//         }
//     }
// }

// /// An iterator over query results.
// pub struct QueryIter<'query, P, Q, F>
// where
//     Q: QueryParams<Guarded = P>,
//     F: FilterParams,
// {
//     world: Arc<World>,
//     entity_iter: EntityIter<'query, F>,
//     locked: &'query LockFlag,
//     index: usize,
//     _marker: PhantomData<(Q, F)>,
// }

// impl<'query, P, F, Q> Iterator for QueryIter<'query, P, Q, F>
// where
//     P: Queryable + 'static,
//     F: FilterParams,
//     Q: QueryParams<Guarded = P>
// {
//     // type Item = P::Ref<'query>;
//     type Item = P::Guard;

//     fn next(&mut self) -> Option<Self::Item> {
//         if Q::IS_ENTITY {
//             assert_eq!(TypeId::of::<P::Ref<'static>>(), TypeId::of::<Entity>());

//             self.entity_iter.next().map(|entity| {
//                 unsafe {
//                     let cast = std::mem::transmute_copy::<Entity, Self::Item>(&entity);
//                     std::mem::forget(entity);

//                     cast
//                 }
//             })
//         } else {
//             // assert_eq!(TypeId::of::<P::Ref<'static>>(), TypeId::of::<&P>());

//             // todo!("filters");

//             let (components, advance) = Q::fetch::<F>(&self.world, self.index, &self.locked);
//             self.index += advance;

//             components

//             // let typeless_store = self.world.components.map.get(&TypeId::of::<Q::NonRef>());

//             // if let Some(store) = typeless_store {
//             //     let typed_store = store
//             //         .value()
//             //         .as_any()
//             //         .downcast_ref::<TypedStorage<P>>()
//             //         .unwrap();

//             //     // Lock the store while QueryIter exists.
//             //     let store_lock = if self.index == 0 {
//             //         // Acquire lock
//             //         self.locked.flag();
//             //         typed_store.storage.read()
//             //     } else {
//             //         // Retrieve lock
//             //         // SAFETY: This is safe because this query has acquired the lock on the first iteration.
//             //         // Additionally, this lock has been forgotten and therefore this thread logically still owns the lock.
//             //         // In case a second iterator is created from the same query, the lock flag will still be set.
//             //         unsafe { typed_store.storage.make_read_guard_unchecked() }
//             //     };

//             //     // Find next entity ID. This also filters out the entities that do not match the filter.
//             //     let store_index = loop {
//             //         let Some(store_index) = typed_store.reverse_map.read().get(self.index).map(|id| *id) else {
//             //             // No more components remaining.
//             //             std::mem::forget(store_lock);
//             //             return None;
//             //         };

//             //         let entity = Entity {
//             //             id: store_index,
//             //             world: self.world.clone()
//             //         };

//             //         if F::filter(&entity) {
//             //             // Matching entity has been found.
//             //             break store_index
//             //         }

//             //         // Continue to next option
//             //         self.index += 1;
//             //     };

//             //     let item = match Q::SHARED {
//             //         true => {
//             //             // ZSTs need different treatment
//             //             if std::mem::size_of::<P>() == 0 {
//             //                 // SAFETY: This is safe because a ZST does not need initialisation.
//             //                 // It is also impossible to construct unconstructable types such as empty
//             //                 // enums since the types have to be constructed to add them to an entity
//             //                 // in the first place.
//             //                 Some(unsafe {
//             //                     MaybeUninit::uninit().assume_init()
//             //                 })
//             //             } else {
//             //                 // println!("{}", std::any::type_name::<P>());
//             //                 // println!("{}", std::any::type_name::<Self::Item>());

//             //                 // SAFETY: This is simply to get around issues with the type system.
//             //                 // An assertion at the start of the iterator ensures that both types below are
//             //                 // equal to each other. The transmuted component will also not have a longer
//             //                 // lifetime than the original because its lifetime will be bounded to the query.
//             //                 // The container of the component lives longer than the query and it can also not
//             //                 // be modified while this query exists.
//             //                 Some(unsafe {
//             //                     // std::mem::transmute_copy::<&P, Self::Item>(&&store_lock[self.index])
//             //                     std::mem::transmute_copy::<&P, Self::Item>(&&store_lock[store_index.0])
//             //                 })
//             //             }
//             //         }
//             //         false => {
//             //             todo!("Single mutable fetch")
//             //         }
//             //     };

//             //     std::mem::forget(store_lock);
//             //     self.index += 1;
//             //     item
//             // } else {
//             //     // This component is not owned by any entity
//             //     None
//             // }
//         }
//     }
// }

// // impl<'query, P1, P2, F, N> Iterator for QueryIter<'query, (P1, P2), N, F>
// //     where
// //         P1: NonRefQueryParam + 'query,
// //         P2: NonRefQueryParam + 'query,
// //         F: FilterParams,
// //         N: QueryParams<NonRef = (P1, P2)>,
// // {
// //     type Item = &'query N::NonRef;

// //     fn next(&mut self) -> Option<Self::Item> {
// //         todo!()
// //     }
// // }

use std::{any::TypeId, marker::PhantomData, sync::{atomic::Ordering, Arc}};

use crate::{Component, EcsError, EcsResult, Entity, EntityIter, FilterParams, TypedStorage, World};

pub trait QueryParams {
    type Fetchable<'query>;

    const MUTABLE: bool;

    fn fetch<'w>(world: &'w Arc<World>, entity: Entity) -> Option<Self::Fetchable<'w>>;
    fn acquire_locks(world: &World) -> EcsResult<()>;
    fn release_locks(world: &World);
}

impl QueryParams for Entity {
    type Fetchable<'query> = Entity;

    const MUTABLE: bool = false;

    fn fetch<'w>(_world: &'w Arc<World>, entity: Entity) -> Option<Self::Fetchable<'w>> {
        Some(entity)
    }

    fn acquire_locks(_world: &World) -> EcsResult<()> { Ok(()) /* Entities require no locks */ }
    fn release_locks(_world: &World) { /* Entities require no locks. */ }
}

impl<T: Component> QueryParams for &T {
    type Fetchable<'query> = &'query T;

    const MUTABLE: bool = false;

    fn fetch<'w>(world: &'w Arc<World>, entity: Entity) -> Option<Self::Fetchable<'w>> {
        debug_assert_eq!(TypeId::of::<&T>(), TypeId::of::<Self::Fetchable<'static>>(), "QueryParams::Fetchable is incorrect type");

        // Instead of keeping track of lock guards like before, we should instead access the components directly.
        // The scheduler will take care of aliasing issues as it will not schedule mutable queries at the same time as aliased ones.

        let type_id = TypeId::of::<T>();
        let typeless = world.components.map.get(&type_id)?;
        let typed: &TypedStorage<T> = typeless
            .value()
            .as_any()
            .downcast_ref()
            .expect("Failed to downcast typeless storage. The wrong storage type has been inserted into component storage");

        let storage_index = *typed.map.get(&entity.id())?.value();
        let lock = typed.storage.read();
        let component = lock.get(storage_index)?;

        let cast = unsafe {
            // SAFETY: The assertion at the beginning of this function guarantees that `Self::Fetchable<'w>` and `&'w T` are the exact same type.
            // hence transmuting between them is safe. Additionally the lifetime of the returned reference is set to 'query as the existence
            // of this query implies that the component storage exist. Creating a query automatically fully locks storage, preventing any changes and therefore
            // reference invalidation.
            std::mem::transmute_copy::<&T, Self::Fetchable<'w>>(&component)
        };

        Some(cast)
    }

    fn release_locks(world: &World) {
        let type_id = TypeId::of::<T>();
        let typeless = world.components.map
            .get(&type_id)
            .expect("Storage to be unlocked does not exist");

        let typed: &TypedStorage<T> = typeless
            .value()
            .as_any()
            .downcast_ref()
            .expect("Failed to downcast typeless storage. The wrong storage type has been inserted into component storage");

        typed.lock.release_read();
    }

    fn acquire_locks(world: &World) -> EcsResult<()> {
        let type_id = TypeId::of::<T>();
        let typeless = world.components.map
            .get(&type_id)
            .expect("Storage to be locked does not exist");

        let typed: &TypedStorage<T> = typeless
            .value()
            .as_any()
            .downcast_ref()
            .expect("Failed to downcast typeless storage. The wrong storage type has been inserted into component storage");

        typed.lock.acquire_read()
    }
}

impl<T: Component> QueryParams for &mut T {
    type Fetchable<'query> = &'query mut T;

    const MUTABLE: bool = true;

    fn fetch<'w>(world: &'w Arc<World>, entity: Entity) -> Option<Self::Fetchable<'w>> {
        debug_assert_eq!(TypeId::of::<&mut T>(), TypeId::of::<Self::Fetchable<'static>>(), "QueryParams::Fetchable is incorrect type");

        // Instead of keeping track of lock guards like before, we should instead access the components directly.
        // The scheduler will take care of aliasing issues as it will not schedule mutable queries at the same time as aliased ones.

        let type_id = TypeId::of::<T>();
        let typeless = world.components.map.get(&type_id)?;
        let typed: &TypedStorage<T> = typeless
            .value()
            .as_any()
            .downcast_ref()
            .expect("Failed to downcast typeless storage. The wrong storage type has been inserted into component storage");

        let storage_index = *typed.map.get(&entity.id())?.value();
        let mut lock = typed.storage.write();
        let component = lock.get_mut(storage_index)?;

        let cast = unsafe {
            // SAFETY: The assertion at the beginning of this function guarantees that `Self::Fetchable<'w>` and `&'w T` are the exact same type.
            // hence transmuting between them is safe. Additionally the lifetime of the returned reference is set to 'query as the existence
            // of this query implies that the component storage exist. Creating a query automatically fully locks storage, preventing any changes and therefore
            // reference invalidation.
            std::mem::transmute_copy::<&mut T, Self::Fetchable<'w>>(&component)
        };

        Some(cast)
    }

    fn acquire_locks(world: &World) -> EcsResult<()> {
        let type_id = TypeId::of::<T>();
        let typeless = world.components.map
            .get(&type_id)
            .expect("Storage to be locked does not exist");

        let typed: &TypedStorage<T> = typeless
            .value()
            .as_any()
            .downcast_ref()
            .expect("Failed to downcast typeless storage. The wrong storage type has been inserted into component storage");

        typed.lock.acquire_write()
    }

    fn release_locks(world: &World) {
        let type_id = TypeId::of::<T>();
        let typeless = world.components.map
            .get(&type_id)
            .expect("Storage to be locked does not exist");

        let typed: &TypedStorage<T> = typeless
            .value()
            .as_any()
            .downcast_ref()
            .expect("Failed to downcast typeless storage. The wrong storage type has been inserted into component storage");

        typed.lock.release_write()
    }
}

impl<Q1: QueryParams, Q2: QueryParams> QueryParams for (Q1, Q2) {
    type Fetchable<'query> = (Q1::Fetchable<'query>, Q2::Fetchable<'query>);

    const MUTABLE: bool = Q1::MUTABLE || Q2::MUTABLE;

    fn fetch<'w>(world: &'w Arc<World>, entity: Entity) -> Option<Self::Fetchable<'w>> {
        let q1 = Q1::fetch(world, entity.clone())?;
        let q2 = Q2::fetch(world, entity)?;

        Some((q1, q2))
    }

    fn acquire_locks(world: &World) -> EcsResult<()> {
        Q1::acquire_locks(world)?;

        if let Err(err) = Q2::acquire_locks(world) {
            Q1::release_locks(world);
            return Err(err)
        }

        Ok(())
    }

    fn release_locks(world: &World) {
        Q1::release_locks(world);
        Q2::release_locks(world);
    }
}

pub struct Query<Q: QueryParams, F: FilterParams = ()> {
    world: Arc<World>,
    /// Use pointer in marker to ensure this type cannot be sent between threads.
    /// 
    /// This is required because when the query is started it obtains a lock on the storages.
    /// A lock should only be used from the thread that owns it. If this query were to be 
    /// transferred to another thread, it would cause undefined behaviour. 
    /// 
    /// I don't see any easy way to make a query thread safe as that would require 
    _marker: PhantomData<*const (Q, F)>
}

unsafe impl<Q: QueryParams, F: FilterParams> Send for Query<Q, F> {}
unsafe impl<Q: QueryParams, F: FilterParams> Sync for Query<Q, F> {}

impl<Q: QueryParams, F: FilterParams> Query<Q, F> {
    pub fn new(world: Arc<World>) -> EcsResult<Self> {
        // Obtain lock on component storage.
        Q::acquire_locks(&world)?;

        Ok(Self { world, _marker: PhantomData })
    }
}

impl<Q: QueryParams, F: FilterParams> Drop for Query<Q, F> {
    fn drop(&mut self) {
        // Locks can be released unconditionally.
        // Whenever this code runs, a query has been created and all locks have therefore been acquired succesfully.
        Q::release_locks(&self.world);
    }
}

impl<'query, Q: QueryParams, F: FilterParams> IntoIterator for &'query Query<Q, F> {
    type Item = Q::Fetchable<'query>;
    type IntoIter = QueryIter<'query, Q, F>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter::from(self)
    }
}

pub struct QueryIter<'query, Q: QueryParams, F: FilterParams> {
    query: &'query Query<Q, F>,
    entities: EntityIter<'query, F>
}

impl<'query, Q: QueryParams, F: FilterParams> Iterator for QueryIter<'query, Q, F> {
    type Item = Q::Fetchable<'query>;

    fn next(&mut self) -> Option<Self::Item> {
        // Obtain the next entity that matches the filter.
        let entity = self.entities.next()?;
        Q::fetch(&self.query.world, entity)
    }
}

impl<'query, Q: QueryParams, F: FilterParams> From<&'query Query<Q, F>> for QueryIter<'query, Q, F> {
    fn from(query: &'query Query<Q, F>) -> Self {
        let entities = query.world.entities.iter(&query.world);
        QueryIter {
            query, entities
        }
    }
}