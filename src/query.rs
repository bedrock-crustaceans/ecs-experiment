use std::any::TypeId;
use std::iter::FusedIterator;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{marker::PhantomData, sync::Arc};

use crate::component::{Component, Components, TypedStorage};
use crate::{Entity, sealed, World};
use crate::sealed::Sealed;

pub trait Filter {}

pub trait NonRefQueryParam {}

impl NonRefQueryParam for Entity {}
impl<T: Component> NonRefQueryParam for T {}

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
}

impl QueryParams for Entity {
    type NonRef = Entity;

    const SHARED: bool = true;
    const IS_ENTITY: bool = true;

    // Requesting entities uses no locking, so this function is empty.
    unsafe fn unlock_all<S: Sealed>(_components: &Components) {}
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

            // Release the component lock
            // SAFETY: Because of the required guarantees made by the caller.
            // Unlocking the read lock specifically is valid because this function is only implemented
            // for shared references, which only utilise shared locks.
            typed_store.storage.force_unlock_read();
        }
    }
}

impl<T: Component> QueryParams for &mut T {
    type NonRef = T;

    const SHARED: bool = false;

    unsafe fn unlock_all<S: sealed::Sealed>(components: &Components) {
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
        todo!()
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
/// #   world.execute().await;
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
pub struct Query<Q: QueryParams, F: FilterBundle = ()> {
    /// Pointer to the world that this query is directed at.
    world: Arc<World>,
    /// Whether this query has acquired locks on component storages.
    locked: LockFlag,
    /// Suppresses the unused parameter errors.
    _marker: PhantomData<(Q, F)>,
}

impl<Q: QueryParams, F: FilterBundle> Query<Q, F> {
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
    P: NonRefQueryParam + 'static,
    F: FilterBundle,
    N: QueryParams<NonRef = P>,
{
    type Item = &'query N::NonRef;
    type IntoIter = QueryIter<'query, P, N, F>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter {
            world: self.world.clone(),
            locked: &self.locked,
            index: 0,
            _marker: PhantomData,
        }
    }
}

impl<Q, F> Drop for Query<Q, F>
where
    Q: QueryParams,
    F: FilterBundle,
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
    F: FilterBundle,
{
    world: Arc<World>,
    locked: &'query LockFlag,
    index: usize,
    _marker: PhantomData<&'query (Q, F)>,
}

impl<'query, P, F, Q> Iterator for QueryIter<'query, P, Q, F>
where
    P: NonRefQueryParam + 'static,
    F: FilterBundle,
    Q: QueryParams<NonRef = P>,
{
    type Item = &'query P;

    fn next(&mut self) -> Option<Self::Item> {
        if Q::IS_ENTITY {

            todo!()
        } else {
            let typeless_store = self.world.components.map.get(&TypeId::of::<Q::NonRef>());

            if let Some(store) = typeless_store {
                let typed_store = store
                    .value()
                    .as_any()
                    .downcast_ref::<TypedStorage<P>>()
                    .unwrap();

                // Lock the store while QueryIter exists.
                let store_lock = if self.index == 0 {
                    // Acquire lock
                    self.locked.flag();
                    typed_store.storage.read()
                } else {
                    // Retrieve lock
                    // SAFETY: This is safe because this query has acquired the lock on the first iteration.
                    // Additionally, this lock has been forgotten and therefore this thread logically still owns the lock.
                    // In case a second iterator is created from the same query, the lock flag will still be set.
                    unsafe { typed_store.storage.make_read_guard_unchecked() }
                };

                let store_index = typed_store.reverse_map.read().get(self.index).map(|id| *id);
                if store_index.is_none() {
                    // No more components remaining.
                    std::mem::forget(store_lock);
                    return None;
                };

                // let store_index = store_index.unwrap();
                let item = match Q::SHARED {
                    true => {
                        Some(unsafe { &*(&store_lock[self.index] as *const Q::NonRef) })
                        // Some(&store_lock[self.index])
                    }
                    false => {
                        todo!("Single mutable fetch")
                    }
                };

                std::mem::forget(store_lock);
                self.index += 1;
                item
            } else {
                // This component is not owned by any entity
                None
            }
        }
    }
}

impl<'query, P1, P2, F, N> Iterator for QueryIter<'query, (P1, P2), N, F>
    where
        P1: NonRefQueryParam + 'query,
        P2: NonRefQueryParam + 'query,
        F: FilterBundle,
        N: QueryParams<NonRef = (P1, P2)>,
{
    type Item = &'query N::NonRef;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
