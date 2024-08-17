use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};
use ecs_derive::Component;
use parking_lot::RwLock;

use crate::entity::Entity;
use crate::{Component, EntityId, Event, EventReader, EventWriter, Query, Res, ResMut, Resource, State, SystemParam, Without, World};
use crate::filter::With;

static GLOBAL: RwLock<Option<&'static Health>> = RwLock::new(None);

fn detection(
    query: Query<(Entity, &Health), Without<Immortal>>,
    mut writer: EventWriter<Killed>
) {
    for (entity, health) in &query {
        if health.0 <= 0.0 {
            writer.write(Killed { entity });
        }
    }
}

fn execution(mut reader: EventReader<Killed>, mut counter: ResMut<KillCounter>) {
    for event in reader.read() {
        counter.0 += 1;       
        println!("Entity {:?} has been killed. {} entities killed so far", event.entity.id(), counter.0);
        event.entity.despawn();
    }
}

fn reader(mut reader: EventReader<Interval>, counter: Res<KillCounter>) {
    println!("Events: {}", reader.len());

    for _ in reader.read() {
        println!("{} entities have been killed so far", counter.0);
    }
}

fn interval_system(query: Query<&mut LastUpdate>, mut writer: EventWriter<Interval>) {
    let update = query.into_iter().next().unwrap();

    if Instant::now().duration_since(update.instant) > Duration::from_millis(1000) {
        update.instant = Instant::now();
        writer.write(Interval);
    }
}

struct SystemState {
    counter: usize
}

fn state_system(mut state: State<SystemState>) {
    state.counter += 1;
}

#[derive(Debug, Component)]
struct LastUpdate { instant: Instant }

#[derive(Debug, Copy, Clone)]
struct Interval;

impl Event for Interval {}

#[derive(Debug)]
struct KillCounter(u32);

impl Resource for KillCounter {}

#[derive(Debug, Component)]
struct Immortal;

#[derive(Debug, Component)]
struct Health(f32);

#[derive(Clone)]
struct Killed {
    entity: Entity
}

impl Event for Killed {}

fn boxer1<P, S, Fut>(fun: S) -> impl Fn(P) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>
where
    P: SystemParam,
    S: Fn(P) -> Fut + 'static,
    Fut: Future<Output = ()> + Send + Sync + 'static,
{
    move |p0| Box::pin(fun(p0))
}

#[tokio::test]
async fn test() {
    let world = World::new();

    // let pinned = boxer1(interval_system);
    // world.add_system(pinned);

    world.spawn(Health(0.0));
    world.spawn(Health(1.0));
    world.spawn(Health(0.0));
    world.spawn((Health(0.0), Immortal));

    world.spawn(LastUpdate { instant: Instant::now() });

    world.add_system(interval_system);
    world.add_system(reader);

    world.add_resource(KillCounter(0));

    world.add_system(detection);
    world.add_system(execution);

    let mut interval = tokio::time::interval(Duration::from_millis(50));
    loop {
        world.tick().await;
        interval.tick().await;        
    }
}
