#[cfg(test)]
mod test;

mod event;
mod query;
mod resource;
mod system;
mod world;

pub trait Component {}

pub trait Filter {}
