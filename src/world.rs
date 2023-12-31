use std::{
    any::{Any, TypeId},
    collections::HashMap,
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc, mem::MaybeUninit,
};
use std::sync::atomic::{AtomicUsize, Ordering};

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::{Component, SysContainer, SpawnBundle, NakedSys, Sys};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(pub(crate) NonZeroUsize);

pub struct EntityMut<'a> {
    world: &'a World,
    id: EntityId,
}

impl EntityMut<'_> {
    pub fn despawn(self) {
        self.world.components.despawn(self.id);
    }
}

pub struct Entities {
    next_index: AtomicUsize
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }

    pub fn alloc(&self) -> EntityId {
        EntityId(NonZeroUsize::new(self.next_index.fetch_add(1, Ordering::Relaxed)).unwrap())
    }
}

impl Default for Entities {
    fn default() -> Entities {
        Entities {
            next_index: AtomicUsize::new(1)
        }
    }
}

pub trait TypelessStorage {
    fn as_any(&self) -> &dyn Any;

    // fn fetch(&self, entity: EntityId) -> *const c_void;
    fn despawn(&self, entity: EntityId) -> bool;
}

pub(crate) struct TypedStorage<T: Component> {
    pub(crate) map: DashMap<EntityId, usize>,
    pub(crate) reverse_map: RwLock<Vec<EntityId>>,
    pub(crate) storage: RwLock<Vec<T>>,
}

impl<T: Component + 'static> TypedStorage<T> {
    pub fn with(entity: EntityId, component: T) -> Arc<dyn TypelessStorage + Send + Sync> {
        Arc::new(Self {
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

    pub fn fetch(&self, entity: EntityId) -> Option<&T> {
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
    pub(crate) map: DashMap<TypeId, Arc<dyn TypelessStorage + Send + Sync>>
}

impl Components {
    pub fn insert<T: Component + 'static>(&self, entity: EntityId, component: T) -> Option<T> {
        let type_id = TypeId::of::<T>();

        if let Some(store) = self.map.get_mut(&type_id) {
            let downcast: &TypedStorage<T> = store.as_any().downcast_ref().unwrap();
            downcast.insert(entity, component)
        } else {
            self.map.insert(type_id, TypedStorage::with(entity, component));
            None
        }
    }

    pub fn despawn(&self, entity: EntityId) {
        self.map.retain(|_, store| !store.despawn(entity) );
    }
}

pub struct Systems {
    storage: RwLock<Vec<Arc<dyn Sys + Send + Sync>>>,
}

impl Systems {
    pub fn new() -> Systems {
        Systems::default()
    }

    pub fn push(&self, system: Arc<dyn Sys + Send + Sync>) {
        self.storage.write().push(system);
    }

    pub fn call(&self, world: &Arc<World>) {
        let lock = self.storage.read();
        lock.iter().for_each(|sys| {
            sys.call(Arc::clone(world));
        });
    }
}

impl Default for Systems {
    fn default() -> Systems {
        Systems {
            storage: RwLock::new(Vec::new()),
        }
    }
}

pub struct World {
    pub(crate) entities: Entities,
    pub(crate) components: Components,
    systems: Systems,
}

impl World {
    pub fn new() -> Arc<World> {
        Arc::new(World::default())
    }

    pub fn spawn<B: SpawnBundle>(&self, bundle: B) -> EntityMut {
        let entity = self.entities.alloc();
        bundle.insert_into(&self.components, entity);

        EntityMut {
            world: self,
            id: entity,
        }
    }

    #[inline]
    pub fn spawn_empty(&self) -> EntityMut {
        self.spawn(())
    }

    pub fn despawn(&self, entity: EntityId) {
        self.components.despawn(entity);
    }

    pub fn system<P, S>(&self, system: S)
    where
        P: Send + Sync + 'static,
        S: NakedSys<P> + Send + Sync + 'static,
        SysContainer<P, S>: Sys,
    {
        let wrapped = Arc::new(system.into_container());
        self.systems.push(wrapped);
    }

    pub fn execute(self: &Arc<Self>) {
        self.systems.call(self);
    }
}

impl Default for World {
    fn default() -> World {
        World {
            entities: Entities::default(),
            components: Components::default(),
            systems: Systems::default(),
        }
    }
}
