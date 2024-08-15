use std::{any::TypeId, marker::PhantomData, sync::{atomic::Ordering, Arc}};

use crate::{Component, EcsError, EcsResult, Entity, EntityIter, FilterParams, TypedStorage, World};

pub trait QueryParams<'w> {
    type Fetchable<'f> where 'w: 'f;

    const MUTABLE: bool;

    fn fetch<'wb>(world: &'wb World<'w>, entity: Entity<'wb, 'w>) -> Option<Self::Fetchable<'wb>>;
    fn acquire_locks(world: &World) -> EcsResult<()>;
    fn release_locks(world: &World);
}

impl<'w> QueryParams<'w> for Entity<'_, 'w> {
    type Fetchable<'f> = Entity<'f, 'w> where 'w: 'f;

    const MUTABLE: bool = false;

    fn fetch<'wb>(_world: &'wb World<'w>, entity: Entity<'wb, 'w>) -> Option<Self::Fetchable<'wb>> {
        Some(entity)
    }

    fn acquire_locks(_world: &World) -> EcsResult<()> { Ok(()) /* Entities require no locks */ }
    fn release_locks(_world: &World) { /* Entities require no locks. */ }
}

impl<'w, T: Component> QueryParams<'w> for &T {
    type Fetchable<'f> = &'f T where 'w: 'f;

    const MUTABLE: bool = false;

    fn fetch<'wb>(world: &'wb World<'w>, entity: Entity<'wb, 'w>) -> Option<Self::Fetchable<'wb>> {
        // debug_assert_eq!(TypeId::of::<&T>(), TypeId::of::<Self::Fetchable<'static>>(), "QueryParams::Fetchable is incorrect type");

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

impl<'w, T: Component> QueryParams<'w> for &mut T {
    type Fetchable<'f> = &'f mut T where 'w: 'f;

    const MUTABLE: bool = true;

    fn fetch<'wb>(world: &'wb World<'w>, entity: Entity<'wb, 'w>) -> Option<Self::Fetchable<'wb>> {
        // debug_assert_eq!(TypeId::of::<&mut T>(), TypeId::of::<Self::Fetchable<'static>>(), "QueryParams::Fetchable is incorrect type");

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

impl<'w, Q1: QueryParams<'w>, Q2: QueryParams<'w>> QueryParams<'w> for (Q1, Q2) {
    type Fetchable<'f> = (Q1::Fetchable<'f>, Q2::Fetchable<'f>) where 'w: 'f;

    const MUTABLE: bool = Q1::MUTABLE || Q2::MUTABLE;

    fn fetch<'wb>(world: &'wb World<'w>, entity: Entity<'wb, 'w>) -> Option<Self::Fetchable<'wb>> {
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

pub struct Query<'wb, 'w, Q: QueryParams<'w>, F: FilterParams = ()> {
    world: &'wb World<'w>,
    /// Use pointer in marker to ensure this type cannot be sent between threads.
    /// 
    /// This is required because when the query is started it obtains a lock on the storages.
    /// A lock should only be used from the thread that owns it. If this query were to be 
    /// transferred to another thread, it would cause undefined behaviour. 
    /// 
    /// I don't see any easy way to make a query thread safe as that would require 
    _marker: PhantomData<&'w (Q, F)>
}

unsafe impl<'wb, 'w, Q: QueryParams<'w>, F: FilterParams> Send for Query<'wb, 'w, Q, F> {}
unsafe impl<'wb, 'w, Q: QueryParams<'w>, F: FilterParams> Sync for Query<'wb, 'w, Q, F> {}

impl<'wb, 'w, Q: QueryParams<'w>, F: FilterParams> Query<'wb, 'w, Q, F> {
    pub fn new(world: &'wb World<'w>) -> EcsResult<Self> {
        // Obtain lock on component storage.
        Q::acquire_locks(world)?;

        Ok(Self { world, _marker: PhantomData })
    }
}

impl<'wb, 'w, Q: QueryParams<'w>, F: FilterParams> Drop for Query<'wb, 'w, Q, F> {
    fn drop(&mut self) {
        // Locks can be released unconditionally.
        // Whenever this code runs, a query has been created and all locks have therefore been acquired succesfully.
        Q::release_locks(&self.world);
    }
}

impl<'wb, 'w, 'q, Q: QueryParams<'w>, F: FilterParams> IntoIterator for &'q Query<'wb, 'w, Q, F> {
    type Item = Q::Fetchable<'q>;
    type IntoIter = QueryIter<'wb, 'w, 'q, Q, F>;

    fn into_iter(self) -> Self::IntoIter {
        QueryIter::from(self)
    }
}

pub struct QueryIter<'wb, 'w, 'q, Q: QueryParams<'w>, F: FilterParams> {
    query: &'q Query<'wb, 'w, Q, F>,
    entities: EntityIter<'wb, 'w, F>
}

impl<'wb, 'w, 'q, Q: QueryParams<'w>, F: FilterParams> Iterator for QueryIter<'wb, 'w, 'q, Q, F> {
    type Item = Q::Fetchable<'q>;

    fn next(&mut self) -> Option<Self::Item> {
        // Obtain the next entity that matches the filter.
        let entity = self.entities.next()?;
        Q::fetch(&self.query.world, entity)
    }
}

impl<'wb, 'w, 'q, Q: QueryParams<'w>, F: FilterParams> From<&'q Query<'wb, 'w, Q, F>> for QueryIter<'wb, 'w, 'q, Q, F> {
    fn from(query: &'q Query<'wb, 'w, Q, F>) -> Self {
        let entities: EntityIter<'wb, 'w, F> = query.world.entities.iter(&query.world);
        QueryIter {
            query, entities
        }
    }
}