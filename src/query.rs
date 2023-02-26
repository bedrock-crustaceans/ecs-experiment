use std::marker::PhantomData;

use crate::{Component, Filter};

pub trait ComponentBundle {
    const MUTABLE: bool;
}

impl<'a, C: Component> ComponentBundle for &'a C {
    const MUTABLE: bool = false;
}

impl<'a, C: Component> ComponentBundle for &'a mut C {
    const MUTABLE: bool = true;
}

impl<'a, C0, C1> ComponentBundle for (C0, C1)
where
    C0: ComponentBundle,
    C1: ComponentBundle,
{
    const MUTABLE: bool = C0::MUTABLE || C1::MUTABLE;
}

pub trait FilterBundle {}

impl FilterBundle for () {}

impl<F: Filter> FilterBundle for F {}

impl<F0, F1> FilterBundle for (F0, F1)
where
    F0: FilterBundle,
    F1: FilterBundle,
{
}

pub struct Query<C: ComponentBundle, F: FilterBundle = ()> {
    collection: C,
    _marker: PhantomData<F>,
}
