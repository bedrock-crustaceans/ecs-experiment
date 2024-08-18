use futures::stream::FuturesUnordered;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{any::TypeId, future::Future, marker::PhantomData, pin::Pin, sync::{atomic::{AtomicBool, AtomicUsize}, Arc}};

use crate::{event::{Event, EventReader, EventWriter}, filter::FilterParams, resource::{Res, ResMut, Resource}, scheduler::{SystemDescriptor, SystemId, SystemParamDescriptor}, sealed, EventState, Query, QueryParams, World};

pub unsafe trait System {
    fn descriptor(&self) -> SystemDescriptor;

    /// # Safety
    /// 
    /// Before running a system you must ensure that the Rust reference aliasing guarantees are upheld.
    /// Any systems requiring mutable access to a component must have unique access.
    fn call(&self, world: &Arc<World>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>;

    /// This function takes a self parameter to make [`System`] object-safe.
    /// 
    /// # Safety
    /// 
    /// This function *must* only return `true` for systems that return the type `Pin<Box<dyn Future<Output = ()> + Send + Sync>>`.
    fn is_async(&self) -> bool;

    /// Runs any preparations before a system's first run.
    /// This is for example used to register all active event readers.
    fn init(&self, world: &Arc<World>) {}
    fn destroy(&self, world: &Arc<World>) {}
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct FnContainer<P: SystemParams, R: SystemReturnable, F: ParameterizedSystem<P, R>> {
    pub id: usize,
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

unsafe impl<P0, R, F: ParameterizedSystem<P0, R>> System for FnContainer<P0, R, F>
where
    P0: SystemParam,
    R: SystemReturnable
{
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: self.id,
            deps: vec![P0::descriptor()]
        }
    }

    fn call(&self, world: &Arc<World>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
        let returned = self.system.call(world, &self.state);
        if self.is_async() {   
            debug_assert_eq!(TypeId::of::<R>(), TypeId::of::<Pin<Box<dyn Future<Output = ()> + Send + Sync>>>());

            // SAFETY: `System::is_async` will only return `true` when `R == Pin<Box<dyn Future<Output = ()> + Send + Sync>>`.
            // It is therefore safe to transmute as both types are equal.
            let cast = unsafe {
                std::mem::transmute_copy::<R, Pin<Box<dyn Future<Output = ()> + Send + Sync>>>(&returned)
            };
            // Prevent dropping the Box, preventing a use-after-free.
            std::mem::forget(returned);

            cast
        } else {
            // Return empty future.
            Box::pin(async {})
        }
    }

    #[inline]
    fn is_async(&self) -> bool {
        R::IS_ASYNC
    }

    fn init(&self, world: &Arc<World>) {
        P0::init(world, &self.state);
    }
}

unsafe impl<P0, P1, R, F: ParameterizedSystem<(P0, P1), R>> System for FnContainer<(P0, P1), R, F>
where
    P0: SystemParam,
    P1: SystemParam,
    R: SystemReturnable
{
    fn descriptor(&self) -> SystemDescriptor {
        SystemDescriptor {
            id: self.id,
            deps: vec![P0::descriptor(), P1::descriptor()]
        }
    }

    fn call(&self, world: &Arc<World>) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
        let returned = self.system.call(world, &self.state);
        if self.is_async() {   
            debug_assert_eq!(TypeId::of::<R>(), TypeId::of::<Pin<Box<dyn Future<Output = ()> + Send + Sync>>>());

            // SAFETY: `System::is_async` will only return `true` when `R == Pin<Box<dyn Future<Output = ()> + Send + Sync>>`.
            // It is therefore safe to transmute as both types are equal.
            let cast = unsafe {
                std::mem::transmute_copy::<R, Pin<Box<dyn Future<Output = ()> + Send + Sync>>>(&returned)
            };
            // Prevent dropping the Box, preventing a use-after-free.
            std::mem::forget(returned);

            cast
        } else {
            // Return empty future.
            Box::pin(async {})
        }
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

    fn descriptor() -> SystemParamDescriptor;

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

    fn descriptor() -> SystemParamDescriptor {
        SystemParamDescriptor::Unit
    }

    fn fetch<S: sealed::Sealed>(_world: &Arc<World>, _state: &Arc<Self::State>) -> Self {}

    fn state(_world: &Arc<World>) -> Arc<Self::State> { Arc::new(()) }
}

pub type PinnedFut = Pin<Box<dyn Future<Output = ()> + Send + Sync + 'static>>;

/// Implemented by async systems to put them into storage containers.
pub trait AsyncSystem<P> where P: SystemParams {
    /// Pins the returned future and puts the system into a container.
    fn pinned(self, id: usize, world: &Arc<World>) -> impl System + Send + Sync + 'static;
}

impl<P, F, Fut> AsyncSystem<P> for F 
where 
    F: Fn(P) -> Fut + Send + Sync + 'static, 
    Fut: Future<Output = ()> + Send + Sync + 'static, 
    P: SystemParam + Send + Sync + 'static
{
    fn pinned(self, id: usize, world: &Arc<World>) -> impl System + Send + Sync + 'static {
        let pinned = move |p1| -> PinnedFut { 
            Box::pin(self(p1))
        };

        pinned.into_container(id, world)
    }
}

impl<P1, P2, F, Fut> AsyncSystem<(P1, P2)> for F 
where 
    F: Fn(P1, P2) -> Fut + Send + Sync + 'static, 
    Fut: Future<Output = ()> + Send + Sync + 'static, 
    P1: SystemParam + Send + Sync + 'static,
    P2: SystemParam + Send + Sync + 'static
{
    fn pinned(self, id: usize, world: &Arc<World>) -> impl System + Send + Sync + 'static {
        let pinned = move |p1, p2| -> PinnedFut {
            Box::pin(self(p1, p2))
        };

        pinned.into_container(id, world)
    }
}

/// Types that can be used as return values for a system.
/// 
/// This is useful for async systems which return futures but is also implemented for unit in the case of sync systems.
/// In the future this could possibly also be used for returning errors or something in that direction.
pub trait SystemReturnable: 'static {
    const IS_ASYNC: bool;
}

impl SystemReturnable for () {
    const IS_ASYNC: bool = false;
}

impl SystemReturnable for Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    const IS_ASYNC: bool = true;
}

pub trait ParameterizedSystem<P: SystemParams, R: SystemReturnable>: Sized {
    fn into_container(self, id: usize, world: &Arc<World>) -> FnContainer<P, R, Self> {
        FnContainer {
            id,
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
        self.storage.write().push(system);
    }

    pub async fn call(&self, world: &Arc<World>) {
        let mut futures = FuturesUnordered::new();

        let lock = self.storage.read();
        for sys_index in 0..self.storage.read().len() {
            let world = Arc::clone(world);
            
            let sys = lock[sys_index].clone();
            futures.push(tokio::spawn(async move {
                sys.call(&world).await;
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
