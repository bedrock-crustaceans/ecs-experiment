use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};
use crate::World;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct EntityId(pub(crate) NonZeroUsize);

pub struct Entity<'a> {
    pub(crate) world: &'a World,
    pub(crate) id: EntityId,
}

impl Entity<'_> {
    pub fn id(&self) -> EntityId {
        self.id
    }

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