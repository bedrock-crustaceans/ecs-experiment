use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use ecs_derive::Component;
use parking_lot::RwLock;

use crate::entity::Entity;
use crate::{Component, EntityId, Event, EventReader, EventWriter, Query, Res, ResMut, Resource, SystemParam, Without, World};
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
    }
}

async fn async_system(query: Query<Entity, With<Immortal>>) {
    for entity in &query {
        println!("Entity {:?}", entity.id());
    }
}

fn boxer1<P, S, Fut>(fun: S) -> impl Fn(P) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>>
where
    P: SystemParam,
    S: Fn(P) -> Fut + 'static,
    Fut: Future<Output = ()> + Send + Sync + 'static,
{
    move |p0| Box::pin(fun(p0))
}

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

#[tokio::test]
async fn test() {
    let world = World::new();

    world.spawn(Health(0.0));
    world.spawn(Health(1.0));
    world.spawn(Health(0.0));
    world.spawn((Health(0.0), Immortal));

    world.add_resource(KillCounter(0));

    let pinned = boxer1(async_system);

    world.add_system(pinned);
    world.add_system(detection);
    world.add_system(execution);

    world.tick().await;
}
