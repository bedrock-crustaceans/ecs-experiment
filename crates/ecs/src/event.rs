use std::{any::{Any, TypeId}, collections::VecDeque, marker::PhantomData, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

use dashmap::DashMap;
use nohash_hasher::{BuildNoHashHasher, NoHashHasher};
use parking_lot::RwLock;

use crate::{World};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct EventId(usize);

struct EventSlot<E: Event> {
    /// Remaining readers that have not seen this event yet.
    /// If this reaches zero, the event is dropped.
    rem: AtomicUsize,
    event: E
}

struct EventTable<E: Event> {
    readers: AtomicUsize,
    next_id: AtomicUsize,
    events: DashMap<usize, EventSlot<E>, BuildNoHashHasher<usize>>
}

impl<E: Event> EventTable<E> {
    pub fn insert(&self, event: E) -> EventId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        self.events.insert(id, EventSlot {
            event, rem: AtomicUsize::new(self.readers.load(Ordering::SeqCst))
        });
        EventId(id)
    }
}

trait EventTableKind: Send + Sync {
    fn clear(&self);
    fn as_any(&self) -> &dyn Any;
}

impl<E: Event> EventTableKind for EventTable<E> {
    fn clear(&self) {
        self.events.clear();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Default)]
pub struct Events {
    storage: DashMap<TypeId, Box<dyn EventTableKind>>
}

impl Events {
    pub fn insert<E: Event>(&self, event: E) -> EventId {
        match self.storage.get(&TypeId::of::<E>()) {
            // Table already exists, insert into existing.
            Some(table) => {
                let table: &EventTable<E> = table
                    .as_any()
                    .downcast_ref()
                    .expect("EventTable type ID does not match event type ID");

                println!("Inserting event");

                table.insert(event)
            },
            // Create new table, it does not exist yet.
            // This case happens when there are no readers for an event.
            None => {
                let table: EventTable<E> = EventTable {
                    readers: AtomicUsize::new(0),
                    next_id: AtomicUsize::new(1), 
                    events: DashMap::with_capacity_and_hasher(1, BuildNoHashHasher::default())
                };

                // There are no readers, so this message will never be read.
                // We can skip adding it to the buffer.

                self.storage.insert(TypeId::of::<E>(), Box::new(table));

                EventId(0)
            }
        }
    }

    pub fn get<E: Event>(&self, id: usize) -> Option<E> {
        let table = self.storage.get(&TypeId::of::<E>())?;
        let table: &EventTable<E> = table.as_any().downcast_ref().expect("EventTable type ID does not match event type ID");

        let slot = table.events.get(&id)?;
        let rem = slot.rem.fetch_sub(1, Ordering::SeqCst);

        if rem == 0 {
            table.events.remove(&id);
        }

        Some(slot.event.clone())
    }

    pub fn add_reader<E: Event>(&self) {
        match self.storage.get(&TypeId::of::<E>()) {
            Some(table) => {
                let table: &EventTable<E> = table
                    .as_any()
                    .downcast_ref()
                    .expect("EventTable type ID does not match event type ID");

                table.readers.fetch_add(1, Ordering::SeqCst);
            },
            None => {
                let table: EventTable<E> = EventTable {
                    readers: AtomicUsize::new(1), next_id: AtomicUsize::new(0), events: DashMap::with_hasher(BuildNoHashHasher::default())
                };

                self.storage.insert(TypeId::of::<E>(), Box::new(table));
            }
        }

        println!("Subscribed reader");
    }

    pub fn remove_reader<E: Event>(&self) {
        let Some(table) = self.storage.get(&TypeId::of::<E>()) else {
            return
        };

        let table: &EventTable<E> = table.as_any().downcast_ref().expect("EventTable type ID does not match event type ID");
        table.readers.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn last_assigned<E: Event>(&self) -> Option<EventId> {
        let table = self.storage.get_mut(&TypeId::of::<E>())?;
        let table: &EventTable<E> = table.as_any().downcast_ref()?;

        Some(EventId(table.next_id.load(Ordering::SeqCst) - 1))
    }
}

pub struct EventWriter<E: Event> {
    world: Arc<World>,
    _marker: PhantomData<E>,
}

impl<E: Event> EventWriter<E> {
    pub(crate) fn new(world: &Arc<World>) -> Self {
        Self { world: Arc::clone(world), _marker: PhantomData }
    }

    pub fn write(&mut self, event: E) -> EventId {
        self.world.events.insert(event)
    }
}

pub struct EventReader<E: Event> {
    world: Arc<World>,
    state: Arc<EventState<E>>,
    _marker: PhantomData<E>,
}

impl<E: Event> EventReader<E> {
    pub(crate) fn new(world: &Arc<World>, state: &Arc<EventState<E>>) -> Self {
        Self { world: Arc::clone(world), state: Arc::clone(state), _marker: PhantomData }
    }

    pub fn len(&self) -> usize {
        let last_assigned = self.world.events.last_assigned::<E>().map(|x| x.0).unwrap_or(0);
        last_assigned - self.state.last_read.load(Ordering::SeqCst)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn read(&mut self) -> EventIterator<E> {
        EventIterator::from(self)
    }

    pub fn par_read(&mut self) -> EventParIterator<E> {
        todo!()
    }
}

pub struct EventIterator<'reader, E: Event> {
    reader: &'reader mut EventReader<E>
}

impl<'reader, E: Event> Iterator for EventIterator<'reader, E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.reader.state.last_read.fetch_add(1, Ordering::SeqCst);
        self.reader.world.events.get(index)
    }
}

impl<'reader, E: Event> From<&'reader mut EventReader<E>> for EventIterator<'reader, E> {
    fn from(reader: &'reader mut EventReader<E>) -> Self {
        Self { reader }
    }
}

pub struct EventParIterator<'reader, E: Event> {
    reader: &'reader mut EventReader<E>
}

pub trait Event: Clone + Send + Sync + 'static {}

pub struct EventState<E: Event> {
    pub last_read: AtomicUsize,
    pub _marker: PhantomData<E>
}