use crate::entity::EntityId;
use crate::{EcsError, EcsResult, LockMarker};
use dashmap::DashMap;
use parking_lot::RwLock;
use std::any::{Any, TypeId};
use std::sync::atomic::{AtomicBool, AtomicU16, AtomicUsize, Ordering};
use std::sync::Arc;
use std::usize;

pub trait Component: Send + Sync + 'static {}

pub trait SpawnBundle {
    fn insert_into(self, components: &Components, entity: EntityId);
}

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

pub trait TypelessStorage: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn remove(&self, entity: EntityId) -> bool;
    fn has_entity(&self, entity: EntityId) -> bool;
}

pub struct TypedStorage<T> {
    pub(crate) lock: LockMarker,

    pub(crate) map: DashMap<EntityId, usize>,
    pub(crate) reverse_map: RwLock<Vec<EntityId>>,
    pub(crate) storage: RwLock<Vec<T>>,
}

impl<T: Send + Sync + 'static> TypedStorage<T> {
    pub fn with(entity: EntityId, component: T) -> Box<dyn TypelessStorage> {
        Box::new(Self {
            lock: LockMarker::new(),
            map: DashMap::from_iter([(entity, 0)]),
            reverse_map: RwLock::new(vec![entity]),
            storage: RwLock::new(vec![component]),
        })
    }

    pub fn insert(&self, entity: EntityId, component: T) -> EcsResult<Option<T>> {
        self.lock.acquire_write()?;

        let result = if let Some(index) = self.map.get(&entity) {
            let mut lock = self.storage.write();
            Ok(Some(std::mem::replace(&mut lock[*index], component)))
        } else {
            let mut lock = self.storage.write();

            let index = lock.len();
            lock.push(component);
            drop(lock);

            self.map.insert(entity, index);
            self.reverse_map.write().push(entity);

            Ok(None)
        };

        self.lock.release_write();

        result
    }
}

impl<T> Default for TypedStorage<T> {
    fn default() -> Self {
        Self {
            lock: LockMarker::new(),
            map: DashMap::new(),
            reverse_map: RwLock::new(Vec::new()),
            storage: RwLock::new(Vec::new()),
        }
    }
}

impl<T: Send + Sync + 'static> TypelessStorage for TypedStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn remove(&self, entity: EntityId) -> bool {
        if let Some((_, index)) = self.map.remove(&entity) {
            let mut lock = self.storage.write();
            // Drop this entity's component from storage and move the tail to its position.
            lock.swap_remove(index);
            if lock.is_empty() {
                // Do not try to remap entities when the last entity is removed.
                // Destroy the reverse mapping instead.
                self.reverse_map.write().clear();
                return false
            }

            // Modify mapping for affected tail entity.
            let mut reverse_lock = self.reverse_map.write();
            let modified_id = reverse_lock[reverse_lock.len() - 1];
            self.map.insert(modified_id, index);
            reverse_lock.swap_remove(index);
        }

        self.storage.read().is_empty()
    }

    fn has_entity(&self, entity: EntityId) -> bool {
        self.map.contains_key(&entity)
    }
}

#[derive(Default)]
pub struct Components {
    pub(crate) map: DashMap<TypeId, Box<dyn TypelessStorage>>,
}

impl Components {
    pub fn insert<T: Component>(&self, entity: EntityId, component: T) -> EcsResult<Option<T>> {
        let type_id = TypeId::of::<T>();

        if let Some(store) = self.map.get_mut(&type_id) {
            let downcast: &TypedStorage<T> = store.as_any().downcast_ref().unwrap();
            downcast.insert(entity, component)
        } else {
            self.map
                .insert(type_id, TypedStorage::with(entity, component));

            Ok(None)
        }
    }

    // /// # Warning
    // pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
    //     let type_id = TypeId::of::<T>();
    //     let store_kv = self.map.get(&type_id)?;
    //     let typed_store: &TypedStorage<T> = store_kv.value().as_any().downcast_ref().unwrap();

    //     todo!()
    // }

    // pub fn get_mut<T: Component>(&self, entity: EntityId) -> Option<&mut T> {
    //     todo!()
    // }

    pub fn has_component<T: Component>(&self, entity: EntityId) -> bool {
        let type_id = TypeId::of::<T>();
        if let Some(store_kv) = self.map.get(&type_id) {
            store_kv.value().has_entity(entity)
        } else {
            false
        }
    }

    pub fn despawn(&self, entity: EntityId) {
        self.map.retain(|_, store| !store.remove(entity));
    }
}
