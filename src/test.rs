use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::RwLock;

use crate::entity::Entity;
use crate::{Component, Query, Resource, Without, World};
use crate::filter::With;

static GLOBAL: RwLock<Option<&'static Health>> = RwLock::new(None);

fn test2(query: Query<Entity, Without<Sleeping>>) {
    // let mut test = None;
    for entity in &query {
        println!("ID: {:?}", entity.id());

        // test = Some(health);
        // *GLOBAL.write() = Some(health);
    }

    // println!("{test:?}");
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

#[derive(Debug)]
struct Sleeping;

impl Component for Sleeping {}

#[derive(Debug)]
struct Health(f32);

impl Component for Health {}

#[tokio::test]
async fn test() {
    let world = World::new();

    world.spawn(Health(0.0));
    world.spawn(Health(1.0));
    world.spawn((Health(2.0), Sleeping));

    world.system(test2);
    // world.system(health_system);
    // world.system(death_system);
    // world.system(sleeping_system);
    // world.system(zst_system);

    world.tick().await;
    // world.tick().await;
}
