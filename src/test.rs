use crate::entity::Entity;
use crate::{Component, Filter, Query, Resource, World};

#[derive(Debug)]
struct Alive;

impl Component for Alive {}

#[derive(Debug)]
struct Health {
    value: f32,
}

impl Component for Health {}

// fn entity_system(query: Query<Entity>) {
//     for entity in &query {
//         entity.remove::<Alive>();
//         dbg!(entity.id);
//     }
// }

fn counter_system(query: Query<Entity>) {
    for counter in &query {
        dbg!(counter.id);
    }
}

#[derive(Debug)]
struct Counter(usize);

impl Component for Counter {}

#[tokio::test]
async fn test() {
    let mut world = World::new();

    let e1 = world.spawn(Counter(1));
    world.spawn(Counter(2));
    world.spawn(Counter(3));

    world.scheduler.post_tick(&world);

    world.system(counter_system);
    world.execute().await;
}
