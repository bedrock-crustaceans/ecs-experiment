use std::marker::PhantomData;

use crate::{
    query::Query,
    resource::{ResMut, Resource},
    world::Systems,
    Component, Filter, World, SystemParamBundle, SystemParam, System,
};

pub struct TestComponent;

impl Component for TestComponent {}

pub struct TestComponent2 {}

impl Component for TestComponent2 {}

pub struct TestFilter<T> {
    _marker: PhantomData<T>,
}

impl<T> Filter for TestFilter<T> {}

pub struct TestFilter2<T> {
    _marker: PhantomData<T>,
}

impl<T> Filter for TestFilter2<T> {}

pub struct TestResource {}

impl Resource for TestResource {}

fn test_system(query: Query<&TestComponent, TestFilter<TestComponent>>) {}

fn mut_test_system(mut query: Query<&mut TestComponent, TestFilter<TestComponent>>) {}

fn tuple_test_system(query: Query<(&TestComponent, &TestComponent2), TestFilter<TestComponent>>) {}

fn tuple_test_system2(
    query: Query<(&TestComponent, &TestComponent2), (TestFilter<TestComponent>, TestFilter2<TestComponent2>)>
) {}

fn res_test_system(query: Query<&TestComponent>, res: ResMut<TestResource>) {}

#[test]
fn test() {
    let mut world = World::new();

    world.system(test_system);
    world.system(mut_test_system);
    world.system(tuple_test_system);
    world.system(tuple_test_system2);
    world.system(res_test_system);

    world.execute();
}
