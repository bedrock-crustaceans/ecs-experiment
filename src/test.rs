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

fn health_system(query: Query<&Health>) {
    for health in &query {
        println!("Entity has {} health", health.value);
        
    }
}

#[tokio::test]
async fn test() {
    let mut world = World::new();
    let e = world.spawn((Alive, Health { value: 1.0 }));

    e.remove::<Alive>();
    // e.remove::<Health>();

    world.scheduler.post_tick(&world);

    world.system(health_system);
    world.execute().await;

    //
    // println!("\n\nSpawn 2 entities");
    // let entity1 = world.spawn((Health { value: 1.0 }, Alive));
    // let entity2 = world.spawn(Health { value: 0.0 });
    //
    // // dbg!(entity1.get::<Health>());
    //
    // world.execute().await;
    //
    // println!("Despawn entity 2");
    // entity2.despawn();
    //
    // world.execute().await;
    //
    // println!();
}
