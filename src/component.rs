use std::any::{Any, TypeId};
use std::sync::Arc;
use dashmap::DashMap;
use parking_lot::RwLock;
use crate::entity::EntityId;

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

pub(crate) trait TypelessStorage: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn despawn(&self, entity: EntityId) -> bool;
}

pub(crate) struct TypedStorage<T: Component> {
    pub(crate) map: DashMap<EntityId, usize>,
    pub(crate) reverse_map: RwLock<Vec<EntityId>>,
    pub(crate) storage: RwLock<Vec<T>>,
}

impl<T: Component + 'static> TypedStorage<T> {
    pub fn with(entity: EntityId, component: T) -> Box<dyn TypelessStorage> {
        Box::new(Self {
            map: DashMap::from_iter([(entity, 0)]),
            reverse_map: RwLock::new(vec![entity]),
            storage: RwLock::new(vec![component])
        })
    }

    pub fn insert(&self, entity: EntityId, component: T) -> Option<T> {
        if let Some(index) = self.map.get(&entity) {
            let mut lock = self.storage.write();
            Some(std::mem::replace(&mut lock[*index], component))
        } else {
            let mut lock = self.storage.write();

            let index = lock.len();
            lock.push(component);
            drop(lock);

            self.map.insert(entity, index);
            self.reverse_map.write().push(entity);

            None
        }
    }

    pub fn get(&self, entity: EntityId) -> Option<&T> {
        todo!()
    }
}

impl<C: Component> Default for TypedStorage<C> {
    fn default() -> Self {
        Self {
            map: DashMap::new(),
            reverse_map: RwLock::new(Vec::new()),
            storage: RwLock::new(Vec::new()),
        }
    }
}

impl<C: Component + 'static> TypelessStorage for TypedStorage<C> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn despawn(&self, entity: EntityId) -> bool {
        if let Some((_, index)) = self.map.remove(&entity) {
            // Drop this entity's component from storage and move the tail to its position.
            self.storage.write().swap_remove(index);
            // Modify mapping for affected tail entity.
            let mut reverse_lock = self.reverse_map.write();
            let modified_id = reverse_lock[reverse_lock.len() - 1];
            self.map.insert(modified_id, index);
            reverse_lock.swap_remove(index);
        }

        self.storage.read().is_empty()
    }
}

#[derive(Default)]
pub struct Components {
    pub(crate) map: DashMap<TypeId, Box<dyn TypelessStorage>>
}

impl Components {
    pub fn insert<T: Component>(&self, entity: EntityId, component: T) -> Option<T> {
        let type_id = TypeId::of::<T>();

        if let Some(store) = self.map.get_mut(&type_id) {
            let downcast: &TypedStorage<T> = store.as_any().downcast_ref().unwrap();
            downcast.insert(entity, component)
        } else {
            self.map.insert(type_id, TypedStorage::with(entity, component));
            None
        }
    }

    pub fn get<T: Component>(&self, entity: EntityId) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        todo!()
    }

    pub fn despawn(&self, entity: EntityId) {
        self.map.retain(|_, store| !store.despawn(entity) );
    }
}