use futures::stream::FuturesUnordered;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{marker::PhantomData, sync::{atomic::AtomicUsize, Arc}};

use crate::{event::{Event, EventReader, EventWriter}, filter::FilterParams, resource::{Res, ResMut, Resource}, sealed, EventState, Query, QueryParams, World};

pub trait System {
    /// # Safety
    /// 
    /// Before running a system you must ensure that the Rust reference aliasing guarantees are upheld.
    /// Any systems requiring mutable access to a component must have unique access.
    unsafe fn call(&self, world: &Arc<World>);

    /// Runs any preparations before a system's first run.
    /// This is for example used to register all active event readers.
    fn init(&self, world: &Arc<World>) {}
    fn destroy(&self, world: &Arc<World>) {}
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct SystemContainer<P: SystemParams, F: ParameterizedSystem<P>> {
    pub system: F,
    pub state: P::ArcState,
    pub _marker: PhantomData<P>,
}

pub trait SystemParams {
    type ArcState: Send + Sync;

    fn state() -> Self::ArcState;
}

impl<P: SystemParam> SystemParams for P {
    type ArcState = Arc<P::State>;

    fn state() -> Self::ArcState {
        P::state()
    }
}

impl<P1: SystemParam, P2: SystemParam> SystemParams for (P1, P2) {
    type ArcState = (Arc<P1::State>, Arc<P2::State>);

    fn state() -> Self::ArcState {
        (P1::state(), P2::state())
    }
}

impl<P0, F: ParameterizedSystem<P0>> System for SystemContainer<P0, F>
where
    P0: SystemParam,
{
    unsafe fn call(&self, world: &Arc<World>) {
        self.system.call(world, &self.state);
    }

    fn init(&self, world: &Arc<World>) {
        P0::init(world, &self.state);
    }
}

impl<P0, P1, F: ParameterizedSystem<(P0, P1)>> System for SystemContainer<(P0, P1), F>
where
    P0: SystemParam,
    P1: SystemParam,
{
    unsafe fn call(&self, world: &Arc<World>) {
        self.system.call(world, &self.state);
    }

    fn init(&self, world: &Arc<World>) {
        P0::init(world, &self.state.0);
        P1::init(world, &self.state.1);
    }
}

pub trait SystemParam {
    type State: Send + Sync;

    const MUTABLE: bool;

    #[doc(hidden)]
    fn fetch<S: sealed::Sealed>(world: &Arc<World>, state: &Arc<Self::State>) -> Self;

    /// Creates a new state.
    fn state() -> Arc<Self::State>;
    /// Initializes the parameter for first-time use.
    fn init(_world: &Arc<World>, _state: &Arc<Self::State>) {}
    /// Deinitializes the parameter for when the system is removed from the ECS.
    fn destroy(_world: &Arc<World>, _state: &Arc<Self::State>) {}
}

impl SystemParam for () {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(_world: &Arc<World>, _state: &Arc<Self::State>) -> Self {}

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<Q: QueryParams, F: FilterParams> SystemParam for Query<Q, F> {
    type State = ();

    const MUTABLE: bool = Q::MUTABLE;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        Query::new(world).expect("Failed to create query")
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<R: Resource> SystemParam for Res<R> {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        Res { world: Arc::clone(world), _marker: PhantomData }
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<R: Resource> SystemParam for ResMut<R> {
    type State = ();

    const MUTABLE: bool = true;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        ResMut { world: Arc::clone(world), _marker: PhantomData }
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<E: Event> SystemParam for EventWriter<E> {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        EventWriter::new(world)
    }

    fn state() -> Arc<Self::State> { Arc::new(()) }
}

impl<E: Event> SystemParam for EventReader<E> {
    type State = EventState<E>;

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, state: &Arc<Self::State>) -> Self {
        EventReader::new(world, state)
    }

    fn state() -> Arc<Self::State> {
        Arc::new(EventState {
            last_read: AtomicUsize::new(0),
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

pub trait ParameterizedSystem<P: SystemParams>: Sized {
    fn into_container(self) -> SystemContainer<P, Self> {
        SystemContainer {
            system: self,
            state: P::state(),
            _marker: PhantomData,
        }
    }

    fn call(&self, world: &Arc<World>, state: &P::ArcState);
}

// impl<F> ParameterizedSystem<()> for F
// where
//     F: Fn(),
// {
//     fn call(&self, _world: Arc<World>) {
//         self();
//     }
// }

impl<F, P0> ParameterizedSystem<P0> for F
where
    F: Fn(P0),
    P0: SystemParam,
{
    fn call(&self, world: &Arc<World>, state: &Arc<P0::State>) {
        let p0 = P0::fetch::<sealed::Sealer>(world, state);
        self(p0);
    }
}

impl<F, P0, P1> ParameterizedSystem<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam,
    P1: SystemParam,
{
    fn call(&self, world: &Arc<World>, state: &<(P0, P1) as SystemParams>::ArcState) {
        let p0 = P0::fetch::<sealed::Sealer>(world, &state.0);
        let p1 = P1::fetch::<sealed::Sealer>(world, &state.1);
        self(p0, p1);
    }
}

pub struct Systems {
    storage: RwLock<Vec<Arc<dyn System + Send + Sync>>>,
}

impl Systems {
    pub fn new() -> Systems {
        Systems::default()
    }

    pub fn push(&self, world: &Arc<World>, system: Arc<dyn System + Send + Sync>) {
        // Initialise system state.
        system.init(world);

        self.storage.write().push(system);
    }

    pub async fn call(&self, world: &Arc<World>) {
        let mut futures = FuturesUnordered::new();

        // FIXME: Reduce the amount of arc cloning required. I could maybe remove it altogether.
        for sys_index in 0..self.storage.read().len() {
            let world = Arc::clone(world);
            futures.push(tokio::spawn(async move {
                let lock = world.systems.storage.read();
                let sys = &lock[sys_index];

                unsafe {
                    sys.call(&world);
                }
            }));
        }

        // Run all futures to completion
        while let Some(_) = futures.next().await {}
    }
}

impl Default for Systems {
    fn default() -> Systems {
        Systems {
            storage: RwLock::new(Vec::new()),
        }
    }
}
