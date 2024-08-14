use std::sync::atomic::{AtomicUsize, Ordering};
use ecs_derive::Component;
use parking_lot::RwLock;

use crate::entity::Entity;
use crate::{Component, Event, EventReader, EventWriter, Query, Resource, Without, World};
use crate::filter::With;

static GLOBAL: RwLock<Option<&'static Health>> = RwLock::new(None);

// fn kill_system(
//     query: Query<(Entity, &Health), Without<Sleeping>>,
//     mut writer: EventWriter<Killed>
// ) {
//     for (entity, health) in &query {
//         if health.0 <= 0.0 {
//             writer.write(Killed { entity });
//         }
//     }
// }

// fn kill_receiver(mut reader: EventReader<Killed>) {
//     for event in reader.read() {
//         println!("Killed entity {:?}", event.entity.id());
//     }
// }

#[derive(Clone)]
struct Message {
    content: &'static str
}

impl Event for Message {}

fn sender(mut writer: EventWriter<Message>) {
    writer.write(Message { content: "Hello World!" });
}

fn receiver(mut reader: EventReader<Message>) {
    for message in reader.read() {
        println!("{}", message.content);
    }
}

// /// Logs the health of all entities.
// fn health_system(query: Query<(&Health, &Sleeping)>) {
//     for (health, _sleeping) in &query {
//         println!("Health: {}", health.0);
//     }
// }

// /// Logs the health all sleeping entities.
// fn sleeping_system(query: Query<&Health, With<Sleeping>>) {
//     for sleeping in &query {
//         println!("Entity with health {} is sleeping", sleeping.0);
//     }
// }

// /// System that kills all sleeping entities.
// fn death_system(query: Query<Entity, With<Sleeping>>) {
//     for entity in &query {
//         println!("Despawning entity {} in next tick", entity.id.0);
//         entity.despawn();
//     }
// }

// fn zst_system(query: Query<&Sleeping>) {
//     for zst in &query {
//         println!("Sleeping");
//     }
// }

#[derive(Debug, Component)]
struct Sleeping;

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

    // world.spawn(Health(0.0));
    // world.spawn(Health(1.0));
    // world.spawn((Health(2.0), Sleeping));

    // world.system(kill_system);
    // world.system(kill_receiver);
    world.system(sender);
    world.system(receiver);

    world.tick().await;
    // world.tick().await;
}
