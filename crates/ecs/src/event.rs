use std::{any::{Any, TypeId}, collections::VecDeque, marker::PhantomData, sync::{atomic::{AtomicUsize, Ordering}, Arc}};

use dashmap::DashMap;
use nohash_hasher::{BuildNoHashHasher, NoHashHasher};
use parking_lot::RwLock;

use crate::{sealed, SystemParam, World};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct EventId(pub(crate) usize);

struct EventSlot<E: Event> {
    /// Remaining readers that have not seen this event yet.
    /// 
    /// Every time a reader reads this event the counter is decreased by one,
    /// destroying the slot when the counter reaches zero.
    rem: AtomicUsize,
    /// The actual event itself.
    event: E
}

struct EventBus<E: Event> {
    /// The amount of readers listening to this bus.
    readers: AtomicUsize,
    /// Next event ID to be assigned.
    next_id: AtomicUsize,
    /// Currently unread events.
    events: DashMap<usize, EventSlot<E>, BuildNoHashHasher<usize>>
}

impl<E: Event> EventBus<E> {
    pub fn insert(&self, event: E) -> EventId {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        println!("next id: {id}");
        self.events.insert(id, EventSlot {
            event, rem: AtomicUsize::new(self.readers.load(Ordering::SeqCst))
        });
        EventId(id)
    }
}

trait EventHolder: Send + Sync {
    fn clear(&self);
    fn as_any(&self) -> &dyn Any;
}

impl<E: Event> EventHolder for EventBus<E> {
    fn clear(&self) {
        self.events.clear();
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[derive(Default)]
pub struct Events {
    storage: DashMap<TypeId, Box<dyn EventHolder>>
}

impl Events {
    pub fn insert<E: Event>(&self, event: E) -> EventId {
        match self.storage.get(&TypeId::of::<E>()) {
            // Table already exists, insert into existing.
            Some(table) => {
                let table: &EventBus<E> = table
                    .as_any()
                    .downcast_ref()
                    .expect("EventTable type ID does not match event type ID");

                table.insert(event)
            },
            // Create new table, it does not exist yet.
            // This case happens when there are no readers for an event.
            None => {
                let table: EventBus<E> = EventBus {
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
        let table: &EventBus<E> = table.as_any().downcast_ref().expect("EventTable type ID does not match event type ID");

        let slot = table.events.get(&id)?;
        let event = slot.event.clone();

        // Release reference into map to allow mutation and prevent deadlock.

        let rem = slot.rem.fetch_sub(1, Ordering::SeqCst);
        drop(slot);

        // AtomicUsize::fetch_sub returns *previous* value so we check for 1 rather than 0.
        if rem == 1 {
            table.events.remove(&id);
        }

        Some(event)
    }

    /// Registers a reader to the specified event bus.
    /// 
    /// This should be done for all systems before running any of them.
    pub fn add_reader<E: Event>(&self) {
        match self.storage.get(&TypeId::of::<E>()) {
            Some(table) => {
                let table: &EventBus<E> = table
                    .as_any()
                    .downcast_ref()
                    .expect("EventTable type ID does not match event type ID");

                table.readers.fetch_add(1, Ordering::SeqCst);
            },
            None => {
                let table: EventBus<E> = EventBus {
                    readers: AtomicUsize::new(1), next_id: AtomicUsize::new(0), events: DashMap::with_hasher(BuildNoHashHasher::default())
                };

                self.storage.insert(TypeId::of::<E>(), Box::new(table));
            }
        }
    }

    /// Unregisters a reader from the specified event bus.
    pub fn remove_reader<E: Event>(&self) {
        let Some(table) = self.storage.get(&TypeId::of::<E>()) else {
            return
        };

        let table: &EventBus<E> = table.as_any().downcast_ref().expect("EventTable type ID does not match event type ID");
        table.readers.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn next_id<E: Event>(&self) -> Option<EventId> {
        let table = self.storage.get_mut(&TypeId::of::<E>())?;
        let table: &EventBus<E> = table.as_any().downcast_ref()?;

        Some(EventId(table.next_id.load(Ordering::SeqCst)))
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

    /// Writes an event into the channel, returning its ID.
    pub fn write(&mut self, event: E) -> EventId {
        self.world.events.insert(event)
    }
}

impl<E: Event> SystemParam for EventWriter<E> {
    type State = ();

    const EXCLUSIVE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        EventWriter::new(world)
    }

    fn state(_world: &Arc<World>) -> Arc<Self::State> { Arc::new(()) }
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

    /// The amount of unread events remaining in this reader.
    pub fn len(&self) -> usize {
        let next_id = self.world.events.next_id::<E>().map(|x| x.0).unwrap_or(0);
        next_id - self.state.last_read.load(Ordering::SeqCst)
    }

    /// Whether this reader has any unread events available.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over unread events.
    pub fn read(&mut self) -> EventIterator<E> {
        EventIterator::from(self)
    }

    pub fn par_read(&mut self) -> EventParIterator<E> {
        todo!()
    }
}

impl<E: Event> SystemParam for EventReader<E> {
    type State = EventState<E>;

    const EXCLUSIVE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, state: &Arc<Self::State>) -> Self {
        EventReader::new(world, state)
    }

    fn state(world: &Arc<World>) -> Arc<Self::State> {
        Arc::new(EventState {
            last_read: AtomicUsize::new(world.events.next_id::<E>().map(|x| x.0).unwrap_or(0)),
            _marker: PhantomData
        })
    }

    fn init(world: &Arc<World>, _state: &Arc<Self::State>) {
        world.events.add_reader::<E>();
    }

    fn destroy(world: &Arc<World>, _state: &Arc<Self::State>) {
        world.events.remove_reader::<E>();
    }
}

pub struct EventIterator<'reader, E: Event> {
    reader: &'reader mut EventReader<E>
}

impl<'reader, E: Event> Iterator for EventIterator<'reader, E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.reader.state.last_read.load(Ordering::SeqCst);
        let item = self.reader.world.events.get(index);
        if item.is_some() {
            self.reader.state.last_read.fetch_add(1, Ordering::SeqCst);
        }

        item
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