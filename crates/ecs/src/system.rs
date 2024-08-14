use futures::stream::FuturesUnordered;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{marker::PhantomData, sync::Arc};

use crate::{event::{Event, EventReader, EventWriter}, filter::FilterParams, resource::{Res, ResMut, Resource}, sealed, Query, QueryParams, World};

pub trait System {
    /// # Safety
    /// 
    /// Before running a system you must ensure that the Rust reference aliasing guarantees are upheld.
    /// Any systems requiring mutable access to a component must have unique access.
    unsafe fn call(&self, world: Arc<World>);
}

struct EventState<E: Event> {
    last_read: usize,
    _marker: PhantomData<E>
}

// pub struct SystemState<> {
//     state
// }

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct SystemContainer<P, F: ParameterizedSystem<P>> {
    pub system: F,
    // pub state: SystemState,
    pub _marker: PhantomData<P>,
}

// trait SystemParams []

impl<F: ParameterizedSystem<()>> System for SystemContainer<(), F> {
    unsafe fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

impl<P0, F: ParameterizedSystem<P0>> System for SystemContainer<P0, F>
where
    P0: SystemParam,
{
    unsafe fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

impl<P0, P1, F: ParameterizedSystem<(P0, P1)>> System for SystemContainer<(P0, P1), F>
where
    P0: SystemParam,
    P1: SystemParam,
{
    unsafe fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

pub trait SystemParam {
    const MUTABLE: bool;

    #[doc(hidden)]
    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self;
}

impl<Q: QueryParams, F: FilterParams> SystemParam for Query<Q, F> {
    const MUTABLE: bool = Q::MUTABLE;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        Query::new(world).expect("Failed to create query")
    }
}

impl<'a, R: Resource> SystemParam for Res<'a, R> {
    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<'a, R: Resource> SystemParam for ResMut<'a, R> {
    const MUTABLE: bool = true;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<E: Event> SystemParam for EventWriter<E> {
    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        EventWriter::new(world)
    }
}

impl<E: Event> SystemParam for EventReader<E> {
    const MUTABLE: bool = false;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        EventReader::new(world)
    }
}

pub trait ParameterizedSystem<Params>: Sized {
    fn into_container(self) -> SystemContainer<Params, Self> {
        SystemContainer {
            system: self,
            _marker: PhantomData,
        }
    }

    fn call(&self, world: Arc<World>);
}

impl<F> ParameterizedSystem<()> for F
where
    F: Fn(),
{
    fn call(&self, _world: Arc<World>) {
        self();
    }
}

impl<F, P0> ParameterizedSystem<P0> for F
where
    F: Fn(P0),
    P0: SystemParam,
{
    fn call(&self, world: Arc<World>) {
        let p0 = P0::fetch::<sealed::Sealer>(world);
        self(p0);
    }
}

impl<F, P0, P1> ParameterizedSystem<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SystemParam,
    P1: SystemParam,
{
    fn call(&self, world: Arc<World>) {
        let p0 = P0::fetch::<sealed::Sealer>(world.clone());
        let p1 = P1::fetch::<sealed::Sealer>(world);
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

    pub fn push(&self, system: Arc<dyn System + Send + Sync>) {
        self.storage.write().push(system);
    }

    pub fn register_state(&self, world: &Arc<World>) {
        for system in self.storage.read().iter() {

        }
    }

    pub async fn call(&self, world: &Arc<World>) {
        let mut futures = FuturesUnordered::new();

        // FIXME: Reduce the amount of arc cloning required. I could maybe remove it altogether.
        for sys_index in 0..self.storage.read().len() {
            let world = Arc::clone(world);
            futures.push(tokio::spawn(async move {
                let lock = world.systems.storage.read();
                let sys = &lock[sys_index];

                let clone = world.clone();
                unsafe {
                    sys.call(clone);
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
