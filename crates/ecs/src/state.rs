use std::{marker::PhantomData, ops::{Deref, DerefMut}, sync::Arc};

use crate::{sealed, SystemParam, World};

pub struct State<S: Send + Sync + Default> {   
    state: Arc<S>,
    _marker: PhantomData<S>
}

impl<S: Send + Sync + Default> SystemParam for State<S> {
    type State = S;

    const EXCLUSIVE: bool = false;

    fn fetch<T: sealed::Sealed>(world: &Arc<World>, state: &Arc<Self::State>) -> Self {
        State { state, _marker: PhantomData }
    }
    
    fn state(_world: &Arc<crate::World>) -> Arc<Self::State> {
        Arc::new(S::default())
    }
}

impl<S: Send + Sync + Default> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

impl<S: Send + Sync + Default> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.state
    }
}