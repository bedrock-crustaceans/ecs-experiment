use futures::stream::FuturesUnordered;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{future::Future, marker::PhantomData, pin::Pin, sync::{atomic::{AtomicBool, AtomicUsize}, Arc}};

use crate::{event::{Event, EventReader, EventWriter}, filter::FilterParams, resource::{Res, ResMut, Resource}, sealed, EventState, Query, QueryParams, World};

pub trait System {
    /// # Safety
    /// 
    /// Before running a system you must ensure that the Rust reference aliasing guarantees are upheld.
    /// Any systems requiring mutable access to a component must have unique access.
    unsafe fn call(&self, world: &Arc<World>);

    /// This function takes a self parameter to make [`System`] object-safe.
    fn is_async(&self) -> bool;

    /// Runs any preparations before a system's first run.
    /// This is for example used to register all active event readers.
    fn init(&self, world: &Arc<World>) {}
    fn destroy(&self, world: &Arc<World>) {}
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct FnContainer<P: SystemParams, R: SystemReturnable, F: ParameterizedSystem<P, R>> {
    pub system: F,
    pub state: P::ArcState,
    pub _marker: PhantomData<(P, R)>,
}

pub trait SystemParams {
    type ArcState: Send + Sync;

    fn state(world: &Arc<World>) -> Self::ArcState;
}

impl<P: SystemParam> SystemParams for P {
    type ArcState = Arc<P::State>;

    fn state(world: &Arc<World>) -> Self::ArcState {
        P::state(world)
    }
}

impl<P1: SystemParam, P2: SystemParam> SystemParams for (P1, P2) {
    type ArcState = (Arc<P1::State>, Arc<P2::State>);

    fn state(world: &Arc<World>) -> Self::ArcState {
        (P1::state(world), P2::state(world))
    }
}

impl<P0, R, F: ParameterizedSystem<P0, R>> System for FnContainer<P0, R, F>
where
    P0: SystemParam,
    R: SystemReturnable
{
    unsafe fn call(&self, world: &Arc<World>) {
        let _returned = self.system.call(world, &self.state);
    }

    #[inline]
    fn is_async(&self) -> bool {
        R::IS_ASYNC
    }

    fn init(&self, world: &Arc<World>) {
        P0::init(world, &self.state);
    }
}

impl<P0, P1, R, F: ParameterizedSystem<(P0, P1), R>> System for FnContainer<(P0, P1), R, F>
where
    P0: SystemParam,
    P1: SystemParam,
    R: SystemReturnable
{
    unsafe fn call(&self, world: &Arc<World>) {
        let _returned = self.system.call(world, &self.state);
    }

    #[inline]
    fn is_async(&self) -> bool {
        R::IS_ASYNC
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
    fn state(world: &Arc<World>) -> Arc<Self::State>;
    /// Initializes the parameter for first-time use.
    fn init(_world: &Arc<World>, _state: &Arc<Self::State>) {}
    /// Deinitializes the parameter for when the system is removed from the ECS.
    fn destroy(_world: &Arc<World>, _state: &Arc<Self::State>) {}
}

impl SystemParam for () {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(_world: &Arc<World>, _state: &Arc<Self::State>) -> Self {}

    fn state(_world: &Arc<World>) -> Arc<Self::State> { Arc::new(()) }
}

impl<Q: QueryParams, F: FilterParams> SystemParam for Query<Q, F> {
    type State = ();

    const MUTABLE: bool = Q::MUTABLE;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        Query::new(world).expect("Failed to create query")
    }

    fn state(_world: &Arc<World>) -> Arc<Self::State> { Arc::new(()) }
}

impl<R: Resource> SystemParam for Res<R> {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        Res { locked: AtomicBool::new(false), world: Arc::clone(world), _marker: PhantomData }
    }

    fn state(_world: &Arc<World>) -> Arc<Self::State> { Arc::new(()) }
}

impl<R: Resource> SystemParam for ResMut<R> {
    type State = ();

    const MUTABLE: bool = true;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        ResMut { locked: AtomicBool::new(false), world: Arc::clone(world), _marker: PhantomData }
    }

    fn state(_world: &Arc<World>) -> Arc<Self::State> { Arc::new(()) }
}

impl<E: Event> SystemParam for EventWriter<E> {
    type State = ();

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, _state: &Arc<Self::State>) -> Self {
        EventWriter::new(world)
    }

    fn state(_world: &Arc<World>) -> Arc<Self::State> { Arc::new(()) }
}

impl<E: Event> SystemParam for EventReader<E> {
    type State = EventState<E>;

    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: &Arc<World>, state: &Arc<Self::State>) -> Self {
        EventReader::new(world, state)
    }

    fn state(world: &Arc<World>) -> Arc<Self::State> {
        Arc::new(EventState {
            last_read: AtomicUsize::new(world.events.last_assigned::<E>().map(|x| x.0).unwrap_or(0)),
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

/// Types that can be used as return values for a system.
/// 
/// This is useful for async systems which return futures but is also implemented for unit in the case of sync systems.
/// In the future this could possibly also be used for returning errors or something in that direction.
pub trait SystemReturnable {
    const IS_ASYNC: bool;
}

impl SystemReturnable for () {
    const IS_ASYNC: bool = false;
}

impl SystemReturnable for Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    const IS_ASYNC: bool = true;
}

pub trait ParameterizedSystem<P: SystemParams, R: SystemReturnable>: Sized {
    fn into_container(self, world: &Arc<World>) -> FnContainer<P, R, Self> {
        FnContainer {
            system: self,
            state: P::state(world),
            _marker: PhantomData,
        }
    }

    fn call(&self, world: &Arc<World>, state: &P::ArcState) -> R;
}

// impl<F> ParameterizedSystem<()> for F
// where
//     F: Fn(),
// {
//     fn call(&self, _world: Arc<World>) {
//         self();
//     }
// }

impl<F, R, P0> ParameterizedSystem<P0, R> for F
where
    F: Fn(P0) -> R,
    P0: SystemParam,
    R: SystemReturnable
{
    fn call(&self, world: &Arc<World>, state: &Arc<P0::State>) -> R {
        let p0 = P0::fetch::<sealed::Sealer>(world, state);
        self(p0)
    }
}

impl<F, R, P0, P1> ParameterizedSystem<(P0, P1), R> for F
where
    F: Fn(P0, P1) -> R,
    P0: SystemParam,
    P1: SystemParam,
    R: SystemReturnable
{
    fn call(&self, world: &Arc<World>, state: &<(P0, P1) as SystemParams>::ArcState) -> R {
        let p0 = P0::fetch::<sealed::Sealer>(world, &state.0);
        let p1 = P1::fetch::<sealed::Sealer>(world, &state.1);
        self(p0, p1)
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

        if system.is_async() {
            println!("Pushing async system");
        }
        
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
