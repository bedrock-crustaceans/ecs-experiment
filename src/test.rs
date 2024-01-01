use crate::entity::Entity;
use crate::{Component, Filter, Query, Resource, World};
use std::marker::PhantomData;

#[derive(Debug)]
struct Alive;

impl Component for Alive {}

#[derive(Debug)]
struct Health {
    value: f32,
}

impl Component for Health {}

fn entity_system(query: Query<Entity>) {
    for entity in &query {
        entity.remove::<Alive>();
        // dbg!(entity.id);
    }
}

#[tokio::test]
async fn test() {
    let mut world = World::new();
    let ent = world.spawn((Alive, Health { value: 1.0 }));

    world.scheduler.post_tick(&world);

    world.system(entity_system);
    world.execute().await;
}
