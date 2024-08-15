use std::{any::TypeId, ops::{Deref, DerefMut}};

use dashmap::DashMap;

#[derive(Default)]
pub struct Resources {
    map: DashMap<TypeId, Box<dyn Resource>>
}

impl Resources {
    pub fn new() -> Self { Self::default() }

    pub fn insert<R: Resource>(&self, resource: R) {
        self.map.insert(TypeId::of::<R>(), Box::new(resource));
    }

    pub fn get<R: Resource>(&self) -> Option<&R> {
        todo!()
    }

    pub fn get_mut<R: Resource>(&self) -> Option<&mut R> {
        todo!()
    }
}

pub trait Resource: Send + Sync + 'static {}

pub struct Res<'res, R> {
    inner: &'res R,
}

impl<'res, R> Deref for Res<'res, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

pub struct ResMut<'a, R> {
    inner: &'a mut R,
}

impl<'res, R> Deref for ResMut<'res, R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl<'res, R> DerefMut for ResMut<'res, R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.inner
    }
}