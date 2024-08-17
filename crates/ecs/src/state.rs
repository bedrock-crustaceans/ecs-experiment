use std::{cell::UnsafeCell, marker::PhantomData, ops::{Deref, DerefMut}, sync::Arc};

use parking_lot::RwLock;

use crate::{sealed, SystemParam, World};

pub struct StateHolder<S: Send + Sync + Default>(UnsafeCell<S>);

unsafe impl<S: Send + Sync + Default> Send for StateHolder<S> {}
unsafe impl<S: Send + Sync + Default> Sync for StateHolder<S> {}

pub struct State<S: Send + Sync + Default> {   
    state: Arc<StateHolder<S>>,
    _marker: PhantomData<S>
}

impl<S: Send + Sync + Default> SystemParam for State<S> {
    type State = StateHolder<S>;

    const EXCLUSIVE: bool = true;

    fn fetch<T: sealed::Sealed>(_world: &Arc<World>, state: &Arc<Self::State>) -> Self {
        State { state: Arc::clone(state), _marker: PhantomData }
    }

    fn state(_world: &Arc<crate::World>) -> Arc<Self::State> {
        Arc::new(StateHolder(UnsafeCell::new(S::default())))
    }
}

impl<S: Send + Sync + Default> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        // SAFETY: A state is unique to each system and therefore can only be referenced by that singular system.
        unsafe { &*self.state.0.get() }
    }
}

impl<S: Send + Sync + Default> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: A state is unique to each system and therefore can only be referenced by that singular system.
        unsafe { &mut *self.state.0.get() }
    }
}