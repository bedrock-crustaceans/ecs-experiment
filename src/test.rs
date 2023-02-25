use std::marker::PhantomData;

use crate::{system::IntoSystemDescriptor, query::Query, Component, QueryFilter};

pub struct TestComponent;

impl Component for TestComponent {

}

pub struct TestComponent2 {

}

impl Component for TestComponent2 {

}

pub struct TestFilter<T> {
    _marker: PhantomData<T>
}

impl<T> QueryFilter for TestFilter<T> {

}

fn test_input<P>(system: impl IntoSystemDescriptor<P>) {

}

fn test_system(query: Query<&TestComponent, TestFilter<TestComponent>>) {
    
}

fn mut_test_system(mut query: Query<&mut TestComponent, TestFilter<TestComponent>>) {
    
}

fn tuple_test_system(query: Query<(&TestComponent, &TestComponent2), TestFilter<TestComponent>>) {
    
}

#[test]
fn test() {
    test_input(test_system);
    test_input(mut_test_system);
    test_input(tuple_test_system);
}