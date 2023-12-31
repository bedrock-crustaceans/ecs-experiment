use std::marker::PhantomData;
use crate::{Component, Filter, Query, Resource, World};

#[derive(Debug)]
pub struct Message1 {
    pub message: &'static str,
}

impl Component for Message1 {}

#[derive(Debug)]
pub struct Message2 {
    pub value: usize
}

impl Component for Message2 {}

pub struct Filter1<T> {
    _marker: PhantomData<T>,
}

impl<T> Filter for Filter1<T> {}

pub struct Filter2<T> {
    _marker: PhantomData<T>,
}

impl<T> Filter for Filter2<T> {}

pub struct Resource1 {}

impl Resource for Resource1 {}

fn test_system(query: Query<&Message2, Filter1<Message1>>) {
    for msg in &query {
        dbg!(msg);
    }
}

#[tokio::test]
async fn test() {
    let mut world = World::new();
    world.spawn(Message2 {
        value: 1
    });
    let entity = world.spawn(Message2 {
        value: 2
    });

    world.system(test_system);
    world.execute();
    entity.despawn();
}
