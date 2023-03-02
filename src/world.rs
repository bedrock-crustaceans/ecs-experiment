use std::{
    any::{Any, TypeId},
    collections::HashMap,
    num::NonZeroUsize,
    rc::Rc,
    sync::Arc, mem::MaybeUninit,
};

use dashmap::DashMap;
use parking_lot::RwLock;

use crate::{Component, SystemObject, InsertionBundle, RawSystem, System};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Entity(NonZeroUsize);

pub struct EntityMut<'a> {
    world: &'a World,
    id: Entity,
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
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        let lock = self.world.entities.storage.read();
        let nth = lock
            .iter()
            .enumerate()
            .find_map(|(i, e)| if *e { Some(i) } else { None })?;

        self.index += nth + 1;
        Some(Entity(NonZeroUsize::new(self.index + 1)?))
    }
}

pub struct Entities {
    storage: RwLock<Vec<bool>>,
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }

    pub fn alloc(&self) -> Entity {
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
        Entity(unsafe { NonZeroUsize::new_unchecked(id) })
    }

    pub fn free(&self, entity: Entity) -> bool {
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

pub trait GenericStorage<C: Component> {
    fn push(&self, entity: Entity, component: C);
    fn fetch(&self, entity: Entity) -> Option<&C>;
}

pub trait Storage {
    fn as_any(&self) -> &dyn Any;
    fn remove(&self, entity: Entity);
}

// pub trait StorageFetch {
//     fn fetch<C: Component>(&self, entity: Entity) -> Option<&C>;
// }

// impl StorageFetch for Arc<dyn Storage + Send + Sync> {
//     fn fetch<C: Component>(&self, entity: Entity) -> Option<&C> {
        
//         // Some(obj)
//     }
// }

pub struct ComponentStorage<C: Component> {
    entity_indices: DashMap<Entity, usize>,
    storage: RwLock<Vec<Option<C>>>,
}

impl<C: Component> ComponentStorage<C> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<C: Component> GenericStorage<C> for ComponentStorage<C> {
    fn push(&self, entity: Entity, component: C) {
        let mut lock = self.storage.write();

        let possible_gap =
            lock.iter()
                .enumerate()
                .find_map(|(i, c)| if c.is_none() { Some(i) } else { None });

        if let Some(gap) = possible_gap {
            self.entity_indices.insert(entity, gap);
            lock[gap] = Some(component);
        } else {
            self.entity_indices.insert(entity, lock.len());
            lock.push(Some(component));
        }
    }

    fn fetch(&self, entity: Entity) -> Option<&C> {
        todo!()
    }
}

impl<C: Component> Default for ComponentStorage<C> {
    fn default() -> Self {
        Self {
            entity_indices: DashMap::new(),
            storage: RwLock::new(Vec::new()),
        }
    }
}

impl<C: Component + 'static> Storage for ComponentStorage<C> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn remove(&self, entity: Entity) {
        if let Some(index) = self.entity_indices.remove(&entity) {
            self.storage.write()[index.1] = None;
        }
    }
}

impl<C: Component> GenericStorage<C> for &Arc<dyn Storage + Send + Sync> {
    fn push(&self, entity: Entity, component: C) {
        
    }

    fn fetch(&self, entity: Entity) -> Option<&C> {
        // <Self as GenericStorage<C>>::fetch(self, entity)
        
    }
}

pub struct Components {
    pub(crate) storage: RwLock<HashMap<TypeId, Arc<dyn Storage + Send + Sync>>>,
}

impl Components {
    pub fn insert_bundle<B: InsertionBundle>(&self, entity: Entity, bundle: B) {
        bundle.insert_into(self, entity);
    }

    pub fn insert<C: Component + 'static>(&self, entity: Entity, component: C) {
        let mut lock = self.storage.write();
        let entry = lock
            .entry(TypeId::of::<C>())
            .or_insert_with(|| Arc::new(ComponentStorage::<C>::new()));

        let downcast = entry.as_any().downcast_ref::<ComponentStorage<C>>();
        if let Some(downcast) = downcast {
            downcast.push(entity, component);
        } else {
            unreachable!();
        }
    }

    pub fn remove_entity(&self, entity: Entity) {
        self.storage.write()
            .iter_mut()
            .for_each(|s| s.1.remove(entity));
    }
}

impl Default for Components {
    fn default() -> Components {
        Components {
            storage: RwLock::new(HashMap::new()),
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

    pub fn spawn<B: InsertionBundle>(&self, bundle: B) -> EntityMut {
        let entity = self.entities.alloc();
        self.components.insert_bundle(entity, bundle);

        EntityMut {
            world: self,
            id: entity,
        }
    }

    pub fn spawn_empty(&self) -> EntityMut {
        let entity = self.entities.alloc();
        EntityMut {
            world: self,
            id: entity,
        }
    }

    pub fn despawn(&self, entity: Entity) -> bool {
        self.components.remove_entity(entity);
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
