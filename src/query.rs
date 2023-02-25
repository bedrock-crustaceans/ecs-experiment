use std::marker::PhantomData;

use crate::{Component, Filter};

pub trait ComponentCollection {

}

impl<'a, T: Component> ComponentCollection for &'a T {

}

impl<'a, T: Component> ComponentCollection for &'a mut T {

}

impl<'a, T0, T1> ComponentCollection for (T0, T1) 
    where T0: ComponentCollection, T1: ComponentCollection
{

}

pub trait FilterCollection {

}

impl FilterCollection for () {

}

impl<T: Filter> FilterCollection for T {

}

impl<T0, T1> FilterCollection for (T0, T1)
    where T0: FilterCollection, T1: FilterCollection
{
    
}

pub struct Query<C: ComponentCollection, F: FilterCollection = ()> {
    collection: C,
    _marker: PhantomData<F>
}

pub struct Res<R> {
    resource: R
}