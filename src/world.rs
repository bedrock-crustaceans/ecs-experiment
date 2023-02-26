use std::num::NonZeroUsize;

use crate::system::System;

pub struct Entity(NonZeroUsize);

pub struct Entities {
    storage: Vec<Option<Entity>>
}

impl Entities {
    pub fn new() -> Entities {
        Entities::default()
    }
}

impl Default for Entities {
    fn default() -> Entities {
        Entities {
            storage: Vec::new()
        }
    }
}

pub struct Components {

}

impl Default for Components {
    fn default() -> Components {
        Components {

        }
    }
}

pub struct Systems {
    storage: Vec<Box<dyn System>>
}

impl Systems {
    pub fn new() -> Systems {
        Systems::default()
    }
}

impl Default for Systems {
    fn default() -> Systems {
        Systems {
            storage: Vec::new()
        }
    }
}

pub struct World {
    entities: Entities,
    components: Components,
    systems: Systems
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