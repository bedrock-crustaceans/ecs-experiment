#[cfg(test)]
mod test;

mod component;
mod entity;
mod event;
mod query;
mod resource;
mod system;
mod world;
mod scheduler;
mod filter;
mod error;
mod util;
mod state;

pub use state::*;
pub use util::*;
pub use error::*;
pub use component::*;
pub use entity::*;
pub use event::*;
pub use query::*;
pub use resource::*;
pub use system::*;
pub use world::*;
pub use filter::*;

fn hello_world() {
    println!("Hello");
}

pub mod prelude {
    use super::component::Component;
    use super::entity::{Entity, EntityId};
    use super::event::{Event, EventId, EventReader, EventWriter};
    use super::query::Query;
    use super::resource::{Resource, Res, ResMut};
    use super::world::World;
    use super::filter::{With, Without, Added, Changed, Removed};
    use super::state::State;
}

pub(crate) mod sealed {
    pub trait Sealed {}
    pub enum Sealer {}

    impl Sealed for Sealer {}
}
