use std::{any::TypeId, marker::PhantomData, ops::Deref};

use crate::{
    query::Query,
    resource::{ResMut, Resource},
    world::Systems,
    Component, Filter, Sys, SysParam, World,
};

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
    for component in query {
        println!("{component:?}");
    }
}

fn mut_test_system(mut query: Query<&mut Message1, Filter1<Message1>>) {}

fn tuple_test_system<'a>(query: Query<'a, (&'a Message1, &'a Message2), Filter1<Message1>>) {}

fn tuple_test_system2(
    query: Query<
        (&Message1, &Message2),
        (Filter1<Message1>, Filter2<Message2>),
    >,
) {
}

fn res_test_system(query: Query<&Message1>, res: ResMut<Resource1>) {}

fn empty_system() {
    println!("Empty system");
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
    // world.system(mut_test_system);
    // world.system(tuple_test_system);
    // world.system(tuple_test_system2);
    // world.system(res_test_system);
    // world.system(empty_system);

    world.execute();
    entity.despawn();
}
