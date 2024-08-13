use std::marker::PhantomData;

#[derive(Default, Debug)]
pub struct EventReader<E: Event> {
    _marker: PhantomData<E>,
}

#[derive(Default, Debug)]
pub struct EventWriter<E: Event> {
    _marker: PhantomData<E>,
}

pub trait Event: Default {}
