use std::{
    any::{Any, TypeId},
    collections::HashMap,
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc, mem::MaybeUninit,
};

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::{Component, SystemObject, SpawnBundle, RawSystem, System};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(NonZeroUsize);

pub struct EntityMut<'a> {
    world: &'a World,
    id: EntityId,
}

impl EntityMut<'_> {
    pub fn despawn(self) {
        self.world.components.despawn(self.id);
    }
}

pub struct EntityIter {
    world: Arc<World>,
    index: usize,
}

impl EntityIter {
    pub fn new(world: Arc<World>) -> Self {
        EntityIter { world, index: 0 }
    }
}

impl Iterator for EntityIter {
    type Item = EntityId;

    fn next(&mut self) -> Option<EntityId> {
        let lock = self.world.entities.storage.read();
        let nth = lock
            .iter()
            .enumerate()
            .find_map(|(i, e)| if *e { Some(i) } else { None })?;

        self.index += nth + 1;
        Some(EntityId(NonZeroUsize::new(self.index + 1)?))
    }
}

pub struct Entities {
    storage: RwLock<Vec<bool>>,
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }

    pub fn alloc(&self) -> EntityId {
        let id = {
            let mut lock = self.storage.write();

            let possible_gap = lock
                .iter()
                .enumerate()
                .find_map(|(i, o)| if !o { Some(i) } else { None });

            if let Some(gap) = possible_gap {
                lock[gap] = true;
                gap + 1
            } else {
                lock.push(true);
                lock.len()
            }
        };

        debug_assert_ne!(id, 0);
        EntityId(unsafe { NonZeroUsize::new_unchecked(id) })
    }

    pub fn free(&self, entity: EntityId) -> bool {
        let mut lock = self.storage.write();

        let mut current = false;
        std::mem::swap(&mut lock[entity.0.get()], &mut current);

        current
    }
}

impl Default for Entities {
    fn default() -> Entities {
        Entities {
            storage: RwLock::new(Vec::new()),
        }
    }
}

pub trait TypelessStorage {
    fn as_any(&self) -> &dyn Any;
    fn despawn(&self, entity: EntityId) -> bool;
}

struct TypedStorage<T: Component> {
    map: DashMap<EntityId, usize>,
    reverse_map: RwLock<Vec<EntityId>>,
    storage: RwLock<Vec<T>>,
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

pub struct Components {
    pub(crate) map: RwLock<HashMap<TypeId, Arc<dyn TypelessStorage + Send + Sync>>>,
}

impl Components {
    pub fn insert<T: Component + 'static>(&self, entity: EntityId, component: T) -> Option<T> {
        let type_id = TypeId::of::<T>();

        let mut lock = self.map.write();
        if let Some(store) = lock.get_mut(&type_id) {
            let downcast: &TypedStorage<T> = store.as_any().downcast_ref().unwrap();
            downcast.insert(entity, component)
        } else {
            lock.insert(type_id, TypedStorage::with(entity, component));
            None
        }
    }

    pub fn despawn(&self, entity: EntityId) {
        self.map.write().retain(|_, store| !store.despawn(entity) );
    }
}

impl Default for Components {
    fn default() -> Components {
        Components {
            map: RwLock::new(HashMap::new()),
        }
    }
}

pub struct Systems {
    storage: RwLock<Vec<Arc<dyn System + Send + Sync>>>,
}

impl Systems {
    pub fn new() -> Systems {
        Systems::default()
    }

    pub fn push(&self, system: Arc<dyn System + Send + Sync>) {
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

    pub fn despawn(&self, entity: EntityId) -> bool {
        self.components.despawn(entity);
        self.entities.free(entity)
    }

    pub fn system<Params, Sys>(&self, system: Sys)
    where
        Params: Send + Sync + 'static,
        Sys: RawSystem<Params> + Send + Sync + 'static,
        SystemObject<Params, Sys>: System,
    {
        let wrapped = Arc::new(system.into_object());
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
