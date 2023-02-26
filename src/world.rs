use std::{
    any::{Any, TypeId},
    collections::HashMap,
    num::NonZeroUsize,
};

use crate::{
    system::{IntoSystem, System, SystemParamBundle},
    Component, GenericSystem, InsertionBundle,
};

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct Entity(NonZeroUsize);

pub struct EntityMut<'w> {
    world: &'w World,
    id: Entity,
}

pub struct Entities {
    storage: Vec<bool>,
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }

    pub fn alloc(&mut self) -> Entity {
        let possible_gap =
            self.storage
                .iter()
                .enumerate()
                .find_map(|(i, o)| if !o { Some(i) } else { None });

        let id = if let Some(gap) = possible_gap {
            self.storage[gap] = true;
            gap + 1
        } else {
            self.storage.push(true);
            self.storage.len()
        };

        debug_assert_ne!(id, 0);
        Entity(unsafe { NonZeroUsize::new_unchecked(id) })
    }
}

impl Default for Entities {
    fn default() -> Entities {
        Entities {
            storage: Vec::new(),
        }
    }
}

pub trait GenericStorage<C: Component> {
    fn push(&mut self, entity: Entity, component: C);
}

pub trait Storage {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct ComponentStorage<C: Component> {
    entity_indices: HashMap<Entity, usize>,
    storage: Vec<C>,
}

impl<C: Component> ComponentStorage<C> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl<C: Component> GenericStorage<C> for ComponentStorage<C> {
    fn push(&mut self, entity: Entity, component: C) {
        self.entity_indices.insert(entity, self.storage.len());
        self.storage.push(component);
    }
}

impl<C: Component> Default for ComponentStorage<C> {
    fn default() -> Self {
        Self {
            entity_indices: HashMap::new(),
            storage: Vec::new(),
        }
    }
}

impl<C: Component + 'static> Storage for ComponentStorage<C> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub struct Components {
    storage: HashMap<TypeId, Box<dyn Storage>>,
}

impl Components {
    pub fn insert_bundle<B: InsertionBundle>(&mut self, entity: Entity, bundle: B) {
        bundle.insert_into(self, entity);
    }

    pub fn insert<C: Component + 'static>(&mut self, entity: Entity, component: C) {
        let entry = self
            .storage
            .entry(TypeId::of::<C>())
            .or_insert_with(|| Box::new(ComponentStorage::<C>::new()));

        let downcast = entry.as_any_mut().downcast_mut::<ComponentStorage<C>>();
        if let Some(downcast) = downcast {
            downcast.push(entity, component);
        } else {
            unreachable!();
        }
    }
}

impl Default for Components {
    fn default() -> Components {
        Components {
            storage: HashMap::new(),
        }
    }
}

pub struct Systems {
    storage: Vec<Box<dyn System>>,
}

impl Systems {
    pub fn new() -> Systems {
        Systems::default()
    }

    pub fn push<S: System + 'static>(&mut self, system: S) {
        self.storage.push(Box::new(system));
    }

    pub fn call(&self) {
        self.storage.iter().for_each(|s| s.call());
    }
}

impl Default for Systems {
    fn default() -> Systems {
        Systems {
            storage: Vec::new(),
        }
    }
}

pub struct World {
    entities: Entities,
    components: Components,
    systems: Systems,
}

impl World {
    pub fn new() -> World {
        World::default()
    }

    pub fn spawn<B: InsertionBundle>(&mut self, bundle: B) -> EntityMut {
        let entity = self.entities.alloc();
        self.components.insert_bundle(entity, bundle);

        EntityMut {
            world: self,
            id: entity,
        }
    }

    pub fn spawn_empty(&mut self) -> EntityMut {
        let entity = self.entities.alloc();
        EntityMut {
            world: self,
            id: entity,
        }
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        todo!()
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
