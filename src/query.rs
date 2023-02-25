use std::marker::PhantomData;

pub trait ComponentCollection {

}

pub trait FilterCollection {

}

pub struct Query<C: ComponentCollection, F: FilterCollection> {
    collection: C,
    _marker: PhantomData<F>
}

pub struct Res<R> {
    resource: R
}