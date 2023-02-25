use std::marker::PhantomData;

use crate::{
    query::Query,
    resource::{Res, ResMut, Resource},
    system::{IntoSystemDescriptor, SystemDescriptor},
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

fn test_input<Params>(system: impl IntoSystemDescriptor<Params>) {
    let descriptor = system.into_descriptor();
    println!("{descriptor:?}");
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
    test_input(test_system);
    test_input(mut_test_system);
    test_input(tuple_test_system);
    test_input(tuple_test_system2);
    test_input(res_test_system);
}
