use std::{any::TypeId, collections::HashMap, num::NonZeroUsize};

use crate::{
    system::{IntoSystem, System, SystemParamBundle},
    Component, ComponentBundle, GenericSystem,
};

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

pub trait GenericStorage<C: Component> {}

pub trait Storage {}

pub struct ComponentStorage<C: Component> {
    entity_indices: HashMap<Entity, usize>,
    storage: Vec<C>,
}

impl<C: Component> GenericStorage<C> for ComponentStorage<C> {}

pub struct Components {
    indices: HashMap<TypeId, usize>,
    storage: Vec<Box<dyn Storage>>,
}

impl Default for Components {
    fn default() -> Components {
        Components {
            ..Default::default()
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

    pub fn spawn<B: ComponentBundle>(&mut self, bundle: B) -> EntityMut {
        let entity = self.entities.alloc();
        todo!("Component insert");

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
            ..Default::default()
        }
    }
}
