use std::sync::atomic::{AtomicUsize, Ordering};
use crate::entity::Entity;
use crate::{Component, Query, Resource, World};
use crate::filter::With;

#[derive(Debug)]
struct Despawn;

impl Component for Despawn {}

#[derive(Debug)]
struct Health {
    value: f32,
}

impl Component for Health {}

fn counter_system(query: Query<&Despawn>) {
    let count = query.into_iter().count();
    println!("There are {count} entities alive");
}

fn despawn_system(query: Query<Entity, With<Despawn>>) {
    for entity in &query {
        println!("Despawning entity {}", entity.id.0);
        entity.despawn();
    }
}

fn despawning_system(query: Query<Entity, With<Despawn>>) {
    query.into_iter().next().map(|entity| entity.despawn());
}

#[derive(Debug)]
struct Counter(usize);

impl Component for Counter {}

#[tokio::test]
async fn test() {
    let mut world = World::new();

    world.spawn((Counter(0)));
    world.spawn((Counter(1)));
    world.spawn((Counter(2), Despawn));

    world.system(despawn_system);

    world.tick().await;
    world.tick().await;
}
