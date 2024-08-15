use std::sync::atomic::{AtomicUsize, Ordering};
use ecs_derive::Component;
use parking_lot::RwLock;

use crate::entity::Entity;
use crate::{Component, EntityId, Event, EventReader, EventWriter, Query, ResMut, Resource, Without, World};
use crate::filter::With;

static GLOBAL: RwLock<Option<&'static Health>> = RwLock::new(None);

fn detection(
    query: Query<(Entity, &Health), Without<Immortal>>,
    mut writer: EventWriter<Killed>
) {
    for (entity, health) in &query {
        if health.0 <= 0.0 {
            writer.write(Killed { entity: entity.id() });
        }
    }
}

fn execution(mut reader: EventReader<Killed>) {
    for event in reader.read() {
        println!("Killing entity {:?}", event.entity);
    }
}

fn counter(mut counter: ResMut<Counter>) {
    println!("{:?}", *counter);
    counter.0 += 1;
}

#[derive(Debug)]
struct Counter(u32);

impl Resource for Counter {}

#[derive(Debug, Component)]
struct Immortal;

#[derive(Debug, Component)]
struct Health(f32);

#[derive(Clone)]
struct Killed {
    entity: EntityId
}

impl Event for Killed {}

#[tokio::test]
async fn test() {
    let world = World::new();

    world.spawn(Health(0.0));
    world.spawn(Health(1.0));
    world.spawn((Health(0.0), Immortal));

    world.add_resource(Counter(0));

    world.add_system(detection);
    world.add_system(execution);
    world.add_system(counter);

    world.tick().await;
    // world.tick().await;
}
