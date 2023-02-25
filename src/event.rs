use std::marker::PhantomData;

pub struct EventReader<E: Event> {
    _marker: PhantomData<E>,
}

pub struct EventWriter<E: Event> {
    _marker: PhantomData<E>,
}

pub trait Event {}
