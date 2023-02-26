use std::num::NonZeroUsize;

use crate::system::{IntoSystem, System, SystemParamBundle};

pub struct Entity(NonZeroUsize);

pub struct Entities {
    storage: Vec<Option<Entity>>,
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }
}

impl Default for Entities {
    fn default() -> Entities {
        Entities {
            storage: Vec::new(),
        }
    }
}

pub struct Components {}

impl Default for Components {
    fn default() -> Components {
        Components {}
    }
}

pub struct Systems {
    storage: Vec<Box<dyn System>>,
}

impl Systems {
    pub fn new() -> Systems {
        Systems::default()
    }

    pub fn push<Params>(&mut self, system: impl IntoSystem<Params> + 'static)
    where
        Params: SystemParamBundle + 'static,
    {
        self.storage.push(Box::new(system.into_system()));
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
}

impl Default for World {
    fn default() -> World {
        World {
            ..Default::default()
        }
    }
}
