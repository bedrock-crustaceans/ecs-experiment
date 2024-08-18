use std::{any::TypeId, marker::PhantomData};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use dashmap::{DashMap, DashSet};
use nohash_hasher::BuildNoHashHasher;
use smallvec::SmallVec;
use crate::{AsyncSystem, Component, EntityId, FnContainer, ParameterizedSystem, System, SystemParams, SystemReturnable, World};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorrowedTypeDescriptor {
    pub exclusive: bool,
    pub type_id: TypeId
}

/// System parameters that the scheduler needs to account for.
/// 
/// Other system parameters such as [`State`](crate::State) or [`EventWriter`](crate::EventWriter)
/// do not have to be accounted for in scheduling because they cannot cause any aliasing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemParamDescriptor {
    Unit,
    EventReader,
    EventWriter,
    State,
    Query(SmallVec<[BorrowedTypeDescriptor; 3]>),
    Res(TypeId),
    ResMut(TypeId),
}

#[derive(Debug)]
pub struct SystemDescriptor {
    pub id: usize,
    pub deps: Vec<SystemParamDescriptor>
}

#[derive(Copy, Clone, PartialEq)]
struct GraphEdge<T: PartialEq + Copy> {
    pub from: T,
    pub to: T
}

impl<T: PartialEq + Copy> From<(T, T)> for GraphEdge<T> {
    fn from((from, to): (T, T)) -> Self {
        Self { from, to }
    }
}

struct OptimizedGraph {

}

#[derive(Default)]
struct ScheduleGraph {
    nodes: Vec<SystemId>,
    edges: Vec<GraphEdge<SystemId>>,
}

impl ScheduleGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: SystemId) {
        self.nodes.push(node);
    }

    pub fn add_edge<I>(&mut self, edge: I) where I: Into<GraphEdge<SystemId>> {
        self.edges.push(edge.into());
    }

    pub fn remove_edge<I>(&mut self, edge: I) where I: Into<GraphEdge<SystemId>> {
        let edge = edge.into();
        self.edges.retain(|x| *x != edge);
    }

    pub fn optimize(self) {
        let mut timeslots = vec![-1; self.nodes.len()];
        let mut available = vec![true; self.nodes.len()];

        for node in self.nodes {

        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct SystemId(usize);

pub trait ExecutorKind {}

pub enum SingleThreadedExecutor {}

impl ExecutorKind for SingleThreadedExecutor {}

pub enum MultiThreadedExecutor {}

impl ExecutorKind for MultiThreadedExecutor {}

pub struct Schedule<K: ExecutorKind> {
    world: Arc<World>,
    next_id: usize,
    systems: HashMap<usize, Arc<dyn System>, BuildNoHashHasher<usize>>,
    _marker: PhantomData<K>
}

impl<K: ExecutorKind> Schedule<K> {
    pub fn new(world: &Arc<World>) -> Self {
        Self { 
            next_id: 0,
            systems: HashMap::with_hasher(BuildNoHashHasher::default()), 
            world: Arc::clone(world), 
            _marker: PhantomData 
        }
    }

    pub fn add_system<P, R, S>(&mut self, system: S) -> SystemId
    where
        P: SystemParams + Send + Sync + 'static,
        R: SystemReturnable + Send + Sync + 'static,
        S: ParameterizedSystem<P, R> + Send + Sync + 'static,
        FnContainer<P, R, S>: System,
    {
        let system_id = self.next_id;
        self.next_id += 1;

        let contained = Arc::new(system.into_container(system_id, &self.world));
        contained.init(&self.world);

        dbg!(contained.descriptor());

        self.systems.insert(system_id, contained);
        SystemId(system_id)
    }

    pub fn add_async_system<P, S>(&mut self, system: S) -> SystemId
    where
        P: SystemParams + Send + Sync + 'static,
        S: AsyncSystem<P>
    {
        let system_id = self.next_id;
        self.next_id += 1;

        let contained = Arc::new(system.pinned(system_id, &self.world));
        contained.init(&self.world);

        dbg!(contained.descriptor());

        self.systems.insert(system_id, contained);
        SystemId(system_id)
    }
}

impl Schedule<SingleThreadedExecutor> {
    fn schedule_graph(&self) -> ScheduleGraph {
        todo!()
    }
}

impl Schedule<MultiThreadedExecutor> {
    fn schedule_graph(&self) -> ScheduleGraph {
        todo!()
    }
}

#[derive(Default)]
pub struct Scheduler {
    /// Keeps track of entities that need to be despawned at the end of a tick.
    despawn_queue: DashSet<EntityId>,
    /// Keeps track of components to remove from entities at the end of a tick.
    remove_queue: DashMap<TypeId, HashSet<EntityId>>
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn schedule_despawn(&self, entity: EntityId) {
        self.despawn_queue.insert(entity);
    }

    pub fn schedule_remove_component(&self, entity: EntityId, type_id: TypeId) {
        let mut entry = self.remove_queue
            .entry(type_id)
            .or_insert_with(HashSet::new);

        entry.value_mut().insert(entity);
    }

    pub fn pre_tick(&self, _world: &Arc<World>) {

    }

    pub fn post_tick(&self, world: &Arc<World>) {
        self.tick_removal(world);
        self.tick_despawn(world);
    }

    fn tick_removal(&self, world: &Arc<World>) {
        self.remove_queue.retain(|type_id, entities| {
            if let Some(store_kv) = world.components.map.get(type_id) {
                for entity in entities.iter() {
                    store_kv.value().remove(*entity);
                }
            }

            false
        });
    }

    fn tick_despawn(&self, world: &Arc<World>) {
        world.entities.free_many(self.despawn_queue.iter().map(|kv| *kv.key()));
        self.despawn_queue
            .retain(|entity| {
                world.components.despawn(*entity);
                false
            });
    }
}