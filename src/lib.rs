#[cfg(test)]
mod test;

mod event;
mod query;
mod resource;
mod system;
mod world;

use std::any::TypeId;

pub use event::*;
pub use query::*;
pub use resource::*;
pub use system::*;
pub use world::*;

pub trait Component: Send + Sync {}

pub trait Filter {}
