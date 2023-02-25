use std::marker::PhantomData;

use crate::{Component, Filter};

pub trait ComponentCollection {
    const MUTABLE: bool;
}

impl<'a, C: Component> ComponentCollection for &'a C {
    const MUTABLE: bool = false;
}

impl<'a, C: Component> ComponentCollection for &'a mut C {
    const MUTABLE: bool = true;
}

impl<'a, C0, C1> ComponentCollection for (C0, C1)
where
    C0: ComponentCollection,
    C1: ComponentCollection,
{
    const MUTABLE: bool = C0::MUTABLE || C1::MUTABLE;
}

pub trait FilterCollection {}

impl FilterCollection for () {}

impl<F: Filter> FilterCollection for F {}

impl<F0, F1> FilterCollection for (F0, F1)
where
    F0: FilterCollection,
    F1: FilterCollection,
{
}

pub struct Query<C: ComponentCollection, F: FilterCollection = ()> {
    collection: C,
    _marker: PhantomData<F>,
}
