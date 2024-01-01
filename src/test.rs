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

fn counter_system(query: Query<&Alive>) {
    let count = query.into_iter().count();
    println!("There are {count} entities alive");
}

fn naming_system(query: Query<Entity>) {
    let entity = query.into_iter().nth(2).unwrap();
    entity.remove::<Alive>();
}

#[derive(Debug)]
struct Counter(usize);

impl Component for Counter {}

#[tokio::test]
async fn test() {
    let mut world = World::new();

    world.spawn((Counter(1), Alive));
    world.spawn((Counter(2), Alive));
    world.spawn((Counter(3), Alive));

    world.system(counter_system);
    world.system(naming_system);

    world.execute().await;
    world.scheduler.post_tick(&world);
    world.execute().await;
}
