use std::sync::atomic::{AtomicUsize, Ordering};
use crate::entity::Entity;
use crate::{Component, Query, Resource, World};
use crate::filter::With;

/// Logs the health of all entities.
fn health_system(query: Query<&Health>) {
    for count in &query {
        // Print the data ins
        println!("Health: {}", count.0);
    }
}

/// Logs the health all sleeping entities.
fn sleeping_system(query: Query<&Health, With<Sleeping>>) {
    for sleeping in &query {
        println!("Entity with health {} is sleeping", sleeping.0);
    }
}

/// System that kills all sleeping entities.
fn death_system(query: Query<Entity, With<Sleeping>>) {
    for entity in &query {
        entity.despawn();
    }
}

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

    world.system(health_system);
    world.system(death_system);
    world.system(sleeping_system);

    world.tick().await;
    world.tick().await;
}
