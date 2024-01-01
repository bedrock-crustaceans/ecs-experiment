#[cfg(test)]
mod test;

mod component;
mod entity;
mod event;
mod query;
mod resource;
mod system;
mod world;

pub use component::*;
pub use entity::*;
pub use event::*;
pub use query::*;
pub use resource::*;
pub use system::*;
pub use world::*;

pub(crate) mod sealed {
    pub trait Sealed {}
    pub enum Sealer {}

    impl Sealed for Sealer {}
}
