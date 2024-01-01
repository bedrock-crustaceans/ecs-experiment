use futures::stream::FuturesUnordered;
use futures::StreamExt;
use parking_lot::RwLock;
use std::{marker::PhantomData, sync::Arc};

use crate::{
    event::{Event, EventReader, EventWriter},
    query::{FilterBundle, Query},
    resource::{Res, ResMut, Resource},
    sealed, QueryParams, World,
};

pub trait Sys {
    fn call(&self, world: Arc<World>);
}

/// Wrapper around a system function pointer to be able to store the function's params.
pub struct SysContainer<P, F: NakedSys<P>> {
    pub system: F,
    pub _marker: PhantomData<P>,
}

impl<F: NakedSys<()>> Sys for SysContainer<(), F> {
    fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

impl<P0, F: NakedSys<P0>> Sys for SysContainer<P0, F>
where
    P0: SysParam,
{
    fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

impl<P0, P1, F: NakedSys<(P0, P1)>> Sys for SysContainer<(P0, P1), F>
where
    P0: SysParam,
    P1: SysParam,
{
    fn call(&self, world: Arc<World>) {
        self.system.call(world);
    }
}

pub trait SysParam {
    const SHARED: bool;

    #[doc(hidden)]
    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self;
}

impl<C: QueryParams, F: FilterBundle> SysParam for Query<C, F> {
    const SHARED: bool = C::SHARED;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        Query::new(world)
    }
}

impl<'a, R: Resource> SysParam for Res<'a, R> {
    const SHARED: bool = false;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<'a, R: Resource> SysParam for ResMut<'a, R> {
    const SHARED: bool = true;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<E: Event> SysParam for EventWriter<E> {
    const SHARED: bool = false;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        todo!();
    }
}

impl<E: Event> SysParam for EventReader<E> {
    const SHARED: bool = false;

    fn fetch<S: sealed::Sealed>(world: Arc<World>) -> Self {
        todo!();
    }
}

pub trait NakedSys<Params>: Sized {
    fn into_container(self) -> SysContainer<Params, Self> {
        SysContainer {
            system: self,
            _marker: PhantomData,
        }
    }

    fn call(&self, world: Arc<World>);
}

impl<F> NakedSys<()> for F
where
    F: Fn(),
{
    fn call(&self, _world: Arc<World>) {
        self();
    }
}

impl<F, P0> NakedSys<P0> for F
where
    F: Fn(P0),
    P0: SysParam,
{
    fn call(&self, world: Arc<World>) {
        let p0 = P0::fetch::<sealed::Sealer>(world);
        self(p0);
    }
}

impl<F, P0, P1> NakedSys<(P0, P1)> for F
where
    F: Fn(P0, P1),
    P0: SysParam,
    P1: SysParam,
{
    fn call(&self, world: Arc<World>) {
        let p0 = P0::fetch::<sealed::Sealer>(world.clone());
        let p1 = P1::fetch::<sealed::Sealer>(world);
        self(p0, p1);
    }
}

pub struct Systems {
    storage: RwLock<Vec<Arc<dyn Sys + Send + Sync>>>,
}

impl Systems {
    pub fn new() -> Systems {
        Systems::default()
    }

    pub fn push(&self, system: Arc<dyn Sys + Send + Sync>) {
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

                let clone = world.clone();
                sys.call(clone);
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
