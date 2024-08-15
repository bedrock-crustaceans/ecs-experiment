use futures::stream::FuturesUnordered;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{marker::PhantomData, sync::{atomic::AtomicUsize, Arc}};

use crate::{event::{Event, EventReader, EventWriter}, filter::FilterParams, resource::{Res, ResMut, Resource}, sealed, EventState, Query, QueryParams, World};

pub trait System<'w> {
    /// # Safety
    /// 
    /// Before running a system you must ensure that the Rust reference aliasing guarantees are upheld.
    /// Any systems requiring mutable access to a component must have unique access.
    unsafe fn call(&self, world: &'w World);

    /// Runs any preparations before a system's first run.
    /// This is for example used to register all active event readers.
    fn init(&self, world: &'w World) {}
    fn destroy(&self, world: &'w World) {}
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct SystemContainer<'w, P: SystemParams<'w>, F: ParameterizedSystem<'w, P>> {
    pub system: F,
    pub state: P::ArcState,
    pub _marker: PhantomData<P>,
}

pub trait SystemParams<'w> {
    type ArcState: Send + Sync;

    fn state() -> Self::ArcState;
}

impl<'w, P: SystemParam<'w>> SystemParams<'w> for P {
    type ArcState = Arc<P::State>;

    fn state() -> Self::ArcState {
        P::state()
    }
}

impl<'w, P1: SystemParam<'w>, P2: SystemParam<'w>> SystemParams<'w> for (P1, P2) {
    type ArcState = (Arc<P1::State>, Arc<P2::State>);

    fn state() -> Self::ArcState {
        (P1::state(), P2::state())
    }
}

impl<'w, P0, F: ParameterizedSystem<'w, P0>> System<'w> for SystemContainer<'w, P0, F>
where
    P0: SystemParam<'w>,
{
    unsafe fn call(&self, world: &'w World) {
        self.system.call(world, &self.state);
    }

    fn init(&self, world: &'w World) {
        P0::init(world, &self.state);
    }
}

impl<'w, P0, P1, F: ParameterizedSystem<'w, (P0, P1)>> System<'w> for SystemContainer<'w, (P0, P1), F>
where
    P0: SystemParam<'w>,
    P1: SystemParam<'w>,
{
    unsafe fn call(&self, world: &'w World) {
        self.system.call(world, &self.state);
    }

    fn init(&self, world: &'w World) {
        P0::init(world, &self.state.0);
        P1::init(world, &self.state.1);
    }
}

pub trait SystemParam<'w> {
    type State: Send + Sync;

    const MUTABLE: bool;

    #[doc(hidden)]
    fn fetch<S: sealed::Sealed>(world: &'w World, state: &Arc<Self::State>) -> Self;

    /// Creates a new state.
    fn state() -> Arc<Self::State>;
    /// Initializes the parameter for first-time use.
    fn init(_world: &'w World, _state: &Arc<Self::State>) {}
    /// Deinitializes the parameter for when the system is removed from the ECS.
    fn destroy(_world: &'w World, _state: &Arc<Self::State>) {}
}

impl<'w> SystemParam<'w> for () {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(_world: &'w World, _state: &Arc<Self::State>) -> Self {}

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<'w, Q: QueryParams, F: FilterParams> SystemParam<'w> for Query<'w, Q, F> {
    type State = ();

    const MUTABLE: bool = Q::MUTABLE;

    fn fetch<S: sealed::Sealed>(world: &'w World, _state: &Arc<Self::State>) -> Self {
        Query::new(world).expect("Failed to create query")
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<'w, R: Resource> SystemParam<'w> for Res<'w, R> {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &'w World, _state: &Arc<Self::State>) -> Self {
        let Some(res) = world.resources.get::<R>() else {
            panic!("Requested resource {} not found, did you forget to add it to the World?", std::any::type_name::<R>());
        };

        todo!()
        // Res { inner: res }
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<'w, R: Resource> SystemParam<'w> for ResMut<'w, R> {
    type State = ();

    const MUTABLE: bool = true;

    fn fetch<S: sealed::Sealed>(world: &'w World, _state: &Arc<Self::State>) -> Self {
        todo!();
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<'w, E: Event> SystemParam<'w> for EventWriter<'w, E> {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &'w World, _state: &Arc<Self::State>) -> Self {
        EventWriter::new(world)
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<'w, E: Event> SystemParam<'w> for EventReader<'w, E> {
    type State = EventState<E>;

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &'w World, state: &Arc<Self::State>) -> Self {
        EventReader::new(world, state)
    }

    fn state() -> Arc<Self::State> {
        Arc::new(EventState {
            last_read: AtomicUsize::new(0),
            _marker: PhantomData
        })
    }

    fn init(world: &'w World, _state: &Arc<Self::State>) {
        world.events.add_reader::<E>();
    }

    fn destroy(world: &'w World, _state: &Arc<Self::State>) {
        world.events.remove_reader::<E>();
    }
}

pub trait ParameterizedSystem<'w, P: SystemParams<'w>>: Sized {
    fn into_container(self) -> SystemContainer<'w, P, Self> {
        SystemContainer {
            system: self,
            state: P::state(),
            _marker: PhantomData,
        }
    }

    fn call(&self, world: &'w World, state: &P::ArcState);
}

// impl<F> ParameterizedSystem<()> for F
// where
//     F: Fn(),
// {
//     fn call(&self, _world: Arc<World>) {
//         self();
//     }
// }

impl<'w, F, P0> ParameterizedSystem<'w, P0> for F
where
    F: Fn(P0),
    P0: SystemParam<'w>,
{
    fn call(&self, world: &'w World, state: &Arc<P0::State>) {
        let p0 = P0::fetch::<sealed::Sealer>(world, state);
        self(p0);
    }
}

impl<'w, F, P0, P1> ParameterizedSystem<'w, (P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam<'w>,
    P1: SystemParam<'w>,
{
    fn call(&self, world: &'w World, state: &<(P0, P1) as SystemParams<'w>>::ArcState) {
        let p0 = P0::fetch::<sealed::Sealer>(world, &state.0);
        let p1 = P1::fetch::<sealed::Sealer>(world, &state.1);
        self(p0, p1);
    }
}

pub struct Systems<'w> {
    storage: RwLock<Vec<Arc<dyn System<'w> + Send + Sync>>>,
}

impl<'w, 'wi> Systems<'w> {
    pub fn new() -> Systems<'w> {
        Systems::default()
    }

    pub fn push(&self, world: &'w World, system: Arc<dyn System<'w> + Send + Sync>) {
        // Initialise system state.
        system.init(world);

        self.storage.write().push(system);
    }

    pub async fn call(&self, world: &'w World<'wi>) {
        // let mut futures = FuturesUnordered::new();

        // FIXME: Reduce the amount of arc cloning required. I could maybe remove it altogether.
        for sys_index in 0..self.storage.read().len() {
            // futures.push(tokio::spawn(async move {
            //     let lock = world.systems.storage.read();
            //     let sys = &lock[sys_index];

            //     unsafe {
            //         sys.call(&world);
            //     }
            // }));
            todo!()
        }

        // Run all futures to completion
        // while let Some(_) = futures.next().await {}
    }
}

impl<'w> Default for Systems<'w> {
    fn default() -> Systems<'w> {
        Systems {
            storage: RwLock::new(Vec::new()),
        }
    }
}
