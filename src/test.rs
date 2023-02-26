use std::marker::PhantomData;

use crate::{
    query::Query,
    resource::{Res, ResMut, Resource},
    system::{IntoSystem, SystemDescriptor, SystemParamCollection},
    world::Systems,
    Component, Filter,
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

fn test_input<Params: SystemParamCollection, S: IntoSystem<Params>>(
    system: S,
) -> SystemDescriptor<Params, S> {
    system.into_descriptor()
    // println!("{descriptor:?}");
}

fn test_system(query: Query<&TestComponent, TestFilter<TestComponent>>) {}

fn mut_test_system(mut query: Query<&mut TestComponent, TestFilter<TestComponent>>) {}

fn tuple_test_system(query: Query<(&TestComponent, &TestComponent2), TestFilter<TestComponent>>) {}

fn tuple_test_system2(
    query: Query<
        (&TestComponent, &TestComponent2),
        (TestFilter<TestComponent>, TestFilter2<TestComponent2>),
    >,
) {
}

fn res_test_system(query: Query<&TestComponent>, res: ResMut<TestResource>) {}

#[test]
fn test() {
    let mut systems = Systems::new();

    let desc1 = test_input(test_system);
    let desc2 = test_input(mut_test_system);
    let desc3 = test_input(tuple_test_system);
    let desc4 = test_input(tuple_test_system2);
    let desc5 = test_input(res_test_system);

    systems.insert(desc1);
    systems.insert(desc2);
    systems.insert(desc3);
    systems.insert(desc4);
    systems.insert(desc5);

    systems.print_all();
}
