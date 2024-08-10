use std::any::TypeId;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{marker::PhantomData, sync::Arc};
use std::mem::MaybeUninit;

use crate::component::{Component, Components, TypedStorage};
use crate::{sealed, Entity, EntityId, EntityIter, Filter, World};
use crate::filter::FilterParams;
use crate::sealed::Sealed;

pub trait DerefQueryParam {
    type Ref<'q>;
}

impl DerefQueryParam for Entity {
    type Ref<'q> = Self;
}

impl<T: Component> DerefQueryParam for T {
    type Ref<'q> = &'q T;
}

impl<T1: Component, T2: Component> DerefQueryParam for (T1, T2) {
    type Ref<'q> = (&'q T1, &'q T2);
}

/// Represents a collection of items contained in a [`Query`].
///
/// Any type that implements this trait can be used in a query.
pub trait QueryParams: Sized {
    /// Represents the type that this trait is implemented for, but with all references removed.
    /// For example: `&Component` becomes `Component`.
    type NonRef;
    /// Whether the query contains only shared references.
    const SHARED: bool;
    /// Whether this type is [`Entity`].
    const IS_ENTITY: bool = false;
    /// Unlocks all used component storages.
    ///
    /// The generic parameter exists to prevent consumers of the crate from calling the function.
    /// This is not done with a private supertrait since that causes all kinds of generic parameter issues
    /// with [`QueryParams`].
    ///
    /// # Safety
    ///
    /// This function should only be called in the Drop implementation of [`Query`] when
    /// the `locked` flag is set to true.
    /// Calling this function in any other situation will lead to undefined behaviour.
    #[doc(hidden)]
    unsafe fn unlock_all<S: sealed::Sealed>(components: &Components);
    /// Fetches the requested components from a world.
    /// 
    /// The `F` generic parameter is there to allow passing filters to the fetch method.
    /// 
    /// # Returns
    /// 
    /// This function returns a tuple of the fetched component(s) and the amount the iterator index should be advanced by.
    fn fetch<F: FilterParams>(world: &Arc<World>, index: usize, lock: &LockFlag) -> (Option<Self>, usize);
}

impl QueryParams for Entity {
    type NonRef = Entity;

    const SHARED: bool = true;
    const IS_ENTITY: bool = true;

    // Requesting entities uses no locking, so this function is empty.
    unsafe fn unlock_all<S: Sealed>(_components: &Components) {}

    fn fetch<F: FilterParams>(world: &Arc<World>, index: usize, lock: &LockFlag) -> (Option<Self>, usize) {
        todo!()
    }
}

impl<T: Component + 'static> QueryParams for &T {
    type NonRef = T;

    const SHARED: bool = true;

    unsafe fn unlock_all<S: sealed::Sealed>(components: &Components) {
        let typeless_store = components.map.get(&TypeId::of::<T>());
        if let Some(store) = typeless_store {
            let typed_store = store
                .value()
                .as_any()
                .downcast_ref::<TypedStorage<T>>()
                .unwrap();

            let locked = typed_store.storage.is_locked();

            // Release the component lock
            // SAFETY: Because of the required guarantees made by the caller.
            // Unlocking the read lock specifically is valid because this function is only implemented
            // for shared references, which only utilise shared locks.
            typed_store.storage.force_unlock_read();
        }
    }

    fn fetch<F: FilterParams>(world: &Arc<World>, mut index: usize, lock: &LockFlag) -> (Option<Self>, usize) {
        let start_index = index;

        let type_id = TypeId::of::<Self::NonRef>();
        let typeless_store = world.components.map.get(&type_id);
            if let Some(store) = typeless_store {
                let typed_store = store
                    .value()
                    .as_any()
                    .downcast_ref::<TypedStorage<T>>()
                    .unwrap();

                // Lock the store while QueryIter exists.
                let store_lock = if !lock.is_flagged() {
                    // Acquire lock
                    lock.flag();
                    typed_store.storage.read()
                } else {
                    // Retrieve lock
                    // SAFETY: This is safe because this query has acquired the lock on the first iteration.
                    // Additionally, this lock has been forgotten and therefore this thread logically still owns the lock.
                    // In case a second iterator is created from the same query, the lock flag will still be set.
                    unsafe { typed_store.storage.make_read_guard_unchecked() }
                };

                // Find next entity ID. This also filters out the entities that do not match the filter.
                let store_index = loop {
                    let Some(store_index) = typed_store.reverse_map.read().get(index).map(|id| *id) else {
                        // No more components remaining.
                        std::mem::forget(store_lock);
                        return (None, index - start_index);
                    };

                    let entity = Entity {
                        id: store_index,
                        world: world.clone()
                    };

                    if F::filter(&entity) {
                        // Matching entity has been found.
                        break store_index
                    }

                    // Continue to next option
                    index += 1;
                };

                // ZSTs need different treatment
                let fetched = if std::mem::size_of::<T>() == 0 {
                    // SAFETY: This is safe because a ZST does not need initialisation.
                    // It is also impossible to construct unconstructable types such as empty
                    // enums since the types have to be constructed to add them to an entity
                    // in the first place.
                    let zst = unsafe {
                        MaybeUninit::uninit().assume_init()
                    };

                    // Creates a static reference to the ZST. As `T` is zero-sized, `Box::new` does not actually allocate.
                    Some(Box::leak(Box::new(zst)) as &T)
                } else {
                    // println!("{}", std::any::type_name::<P>());
                    // println!("{}", std::any::type_name::<Self::Item>());

                    // SAFETY: This is simply to get around issues with the type system.
                    // An assertion at the start of the iterator ensures that both types below are
                    // equal to each other. The transmuted component will also not have a longer
                    // lifetime than the original because its lifetime will be bounded to the query.
                    // The container of the component lives longer than the query and it can also not
                    // be modified while this query exists.
                    Some(unsafe {
                        // std::mem::transmute_copy::<&P, Self::Item>(&&store_lock[self.index])
                        std::mem::transmute_copy::<&T, _>(&&store_lock[store_index.0])
                        // &store_lock[store_index.0]
                    })
                };

                std::mem::forget(store_lock);
                index += 1;
                
                (fetched, index - start_index)
            } else {
                // This component is not owned by any entity
                (None, index - start_index)
            }
    }
}

impl<T: Component> QueryParams for &mut T {
    type NonRef = T;

    const SHARED: bool = false;

    unsafe fn unlock_all<S: sealed::Sealed>(components: &Components) {
        println!("Unlock write");

        let typeless_store = components.map.get(&TypeId::of::<T>());
        if let Some(store) = typeless_store {
            let typed_store = store
                .value()
                .as_any()
                .downcast_ref::<TypedStorage<T>>()
                .unwrap();

            // Release the component lock
            // SAFETY: Because of the required guarantees made by the caller.
            // Unlocking the write lock specifically is valid because this function is only implemented
            // for mutable references, which only utilise write locks.
            typed_store.storage.force_unlock_write();
        }
    }

    fn fetch<F: FilterParams>(world: &Arc<World>, index: usize, lock: &LockFlag) -> (Option<Self>, usize) {
        todo!()
    }
}

impl<T1, T2> QueryParams for (T1, T2)
where
    T1: QueryParams,
    T2: QueryParams,
{
    type NonRef = (T1::NonRef, T2::NonRef);

    const SHARED: bool = T1::SHARED || T2::SHARED;

    unsafe fn unlock_all<S: sealed::Sealed>(components: &Components) {
        T1::unlock_all::<sealed::Sealer>(components);
        T2::unlock_all::<sealed::Sealer>(components);
    }

    fn fetch<F: FilterParams>(world: &Arc<World>, mut index: usize, lock: &LockFlag) -> (Option<Self>, usize) {
        println!("Fetching {} and {}", std::any::type_name::<T1>(), std::any::type_name::<T2>());

        let start = index;

        let (t1, adv) = T1::fetch::<F>(world, index, lock);
        index += adv;

        let Some(t1) = t1 else {
            return (None, index - start)
        };

        println!("Fetched T1");

        let (t2, adv) = T2::fetch::<F>(world, index, lock);
        index += adv;

        let Some(t2) = t2 else {
            return (None, index - start)
        };

        println!("Fetched T2");

        (Some((t1, t2)), index - start)
    }
}

/// For safety reasons, the locked flag in a query should never be set to false once it has been
/// set to true. This type enforces that rule by not providing any methods to set it to false.
#[derive(Default)]
struct LockFlag {
    flag: AtomicBool,
}

impl LockFlag {
    /// Signals the flag.
    #[inline]
    pub fn flag(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    /// Returns whether this flag has been flagged.
    #[inline]
    pub fn is_flagged(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// A query consists of resources and filters. The first generic parameter of the query contains
/// the resources. This can for example be a type implementing [`Component`]
/// or a [`Res`](crate::resource::Res).
/// Additionally, tuples of mixed types are also allowed to request multiple resources at once.
///
/// # Example
/// ```rust
/// # use ecs::{Component, World, Query};
/// # struct Health { value: f32 }
/// # impl Component for Health {}
/// #
/// # #[tokio::main]
/// # async fn main() {
/// #   let world = World::new();
/// #   world.system(health_display);
/// #   world.spawn(Health { value: 1.0 });
/// #   world.tick().await;
/// # }
/// fn health_display(query: Query<&Health>) {
///     for health in &query {
///         println!("Entity has {} health points", health.value);
///     }
/// }
/// ```
/// This example requests the `Health` component and a reference to every entity that has the
/// `Alive` component.
///
/// # Concurrency
/// The query is created in an unlocked state and can be turned into an iterator.
/// Once the iterator performs its first iteration, the query turns into a locked query.
/// This means that all component storages that the query requires will be locked.
/// Whether this constitutes a shared lock or exclusive lock depends on the content of the query.
pub struct Query<Q: QueryParams, F: FilterParams = ()> {
    /// Pointer to the world that this query is directed at.
    world: Arc<World>,
    /// Whether this query has acquired locks on component storages.
    locked: LockFlag,
    /// Suppresses the unused parameter errors.
    _marker: PhantomData<(Q, F)>,
}

impl<Q: QueryParams, F: FilterParams> Query<Q, F> {
    /// Creates a new unlocked query for the specified world.
    pub(crate) fn new(world: Arc<World>) -> Self {
        Self {
            world,
            locked: LockFlag::default(),
            _marker: PhantomData,
        }
    }
}

impl<'query, P, F, N> IntoIterator for &'query Query<N, F>
where
    P: DerefQueryParam + 'static,
    F: FilterParams,
    N: QueryParams<NonRef = P>,
{
    // type Item = P::Ref<'query>;
    type Item = N;
    type IntoIter = QueryIter<'query, P, N, F>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter {
            world: self.world.clone(),
            entity_iter: self.world.entities.iter(self.world.clone()),
            locked: &self.locked,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<Q, F> Drop for Query<Q, F>
where
    Q: QueryParams,
    F: FilterParams,
{
    fn drop(&mut self) {
        if self.locked.is_flagged() {
            // SAFETY: This is safe because the lock flag has been flagged.
            // This flag means that the store of every component type is currently locked.
            // It is therefore to unlock all of the stores.
            unsafe { Q::unlock_all::<sealed::Sealer>(&self.world.components) }
        }
    }
}

/// An iterator over query results.
pub struct QueryIter<'query, P, Q, F>
where
    Q: QueryParams<NonRef = P>,
    F: FilterParams,
{
    world: Arc<World>,
    entity_iter: EntityIter<'query, F>,
    locked: &'query LockFlag,
    index: usize,
    _marker: PhantomData<(Q, F)>,
}

impl<'query, P, F, Q> Iterator for QueryIter<'query, P, Q, F>
where
    P: DerefQueryParam + 'static,
    F: FilterParams,
    Q: QueryParams<NonRef = P>
{
    // type Item = P::Ref<'query>;
    type Item = Q;

    fn next(&mut self) -> Option<Self::Item> {
        if Q::IS_ENTITY {
            assert_eq!(TypeId::of::<P::Ref<'static>>(), TypeId::of::<Entity>());

            self.entity_iter.next().map(|entity| {
                unsafe {
                    let cast = std::mem::transmute_copy::<Entity, Self::Item>(&entity);
                    std::mem::forget(entity);

                    cast
                }
            })
        } else {
            // assert_eq!(TypeId::of::<P::Ref<'static>>(), TypeId::of::<&P>());

            // todo!("filters");

            let (components, advance) = Q::fetch::<F>(&self.world, self.index, &self.locked);
            self.index += advance;

            components

            // let typeless_store = self.world.components.map.get(&TypeId::of::<Q::NonRef>());

            // if let Some(store) = typeless_store {
            //     let typed_store = store
            //         .value()
            //         .as_any()
            //         .downcast_ref::<TypedStorage<P>>()
            //         .unwrap();

            //     // Lock the store while QueryIter exists.
            //     let store_lock = if self.index == 0 {
            //         // Acquire lock
            //         self.locked.flag();
            //         typed_store.storage.read()
            //     } else {
            //         // Retrieve lock
            //         // SAFETY: This is safe because this query has acquired the lock on the first iteration.
            //         // Additionally, this lock has been forgotten and therefore this thread logically still owns the lock.
            //         // In case a second iterator is created from the same query, the lock flag will still be set.
            //         unsafe { typed_store.storage.make_read_guard_unchecked() }
            //     };

            //     // Find next entity ID. This also filters out the entities that do not match the filter.
            //     let store_index = loop {
            //         let Some(store_index) = typed_store.reverse_map.read().get(self.index).map(|id| *id) else {
            //             // No more components remaining.
            //             std::mem::forget(store_lock);
            //             return None;
            //         };

            //         let entity = Entity {
            //             id: store_index,
            //             world: self.world.clone()
            //         };

            //         if F::filter(&entity) {
            //             // Matching entity has been found.
            //             break store_index
            //         }

            //         // Continue to next option
            //         self.index += 1;
            //     };

            //     let item = match Q::SHARED {
            //         true => {
            //             // ZSTs need different treatment
            //             if std::mem::size_of::<P>() == 0 {
            //                 // SAFETY: This is safe because a ZST does not need initialisation.
            //                 // It is also impossible to construct unconstructable types such as empty
            //                 // enums since the types have to be constructed to add them to an entity
            //                 // in the first place.
            //                 Some(unsafe {
            //                     MaybeUninit::uninit().assume_init()
            //                 })
            //             } else {
            //                 // println!("{}", std::any::type_name::<P>());
            //                 // println!("{}", std::any::type_name::<Self::Item>());

            //                 // SAFETY: This is simply to get around issues with the type system.
            //                 // An assertion at the start of the iterator ensures that both types below are
            //                 // equal to each other. The transmuted component will also not have a longer
            //                 // lifetime than the original because its lifetime will be bounded to the query.
            //                 // The container of the component lives longer than the query and it can also not
            //                 // be modified while this query exists.
            //                 Some(unsafe {
            //                     // std::mem::transmute_copy::<&P, Self::Item>(&&store_lock[self.index])
            //                     std::mem::transmute_copy::<&P, Self::Item>(&&store_lock[store_index.0])
            //                 })
            //             }
            //         }
            //         false => {
            //             todo!("Single mutable fetch")
            //         }
            //     };

            //     std::mem::forget(store_lock);
            //     self.index += 1;
            //     item
            // } else {
            //     // This component is not owned by any entity
            //     None
            // }
        }
    }
}

// impl<'query, P1, P2, F, N> Iterator for QueryIter<'query, (P1, P2), N, F>
//     where
//         P1: NonRefQueryParam + 'query,
//         P2: NonRefQueryParam + 'query,
//         F: FilterParams,
//         N: QueryParams<NonRef = (P1, P2)>,
// {
//     type Item = &'query N::NonRef;

//     fn next(&mut self) -> Option<Self::Item> {
//         todo!()
//     }
// }
