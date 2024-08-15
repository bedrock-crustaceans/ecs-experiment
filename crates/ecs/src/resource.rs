use std::{any::{Any, TypeId}, cell::UnsafeCell, marker::PhantomData, ops::{Deref, DerefMut}, sync::{atomic::{AtomicBool, Ordering}, Arc}};

use dashmap::DashMap;

use crate::{EcsError, EcsResult, LockMarker, World};

struct ResourceSingleton<R: Resource> {
    lock_marker: LockMarker,
    resource: UnsafeCell<R>
}

unsafe impl<R: Resource> Send for ResourceSingleton<R> {}
unsafe impl<R: Resource> Sync for ResourceSingleton<R> {}

impl<R: Resource> ResourceSingleton<R> {
    pub fn acquire_read(&self) -> EcsResult<()> {
        self.lock_marker.acquire_read()
    }

    pub fn release_read(&self) {
        self.lock_marker.release_read();
    }

    /// # Safety:
    /// 
    /// Aliasing invariants must be upheld manually.
    /// Ensure you've acauired the appropriate lock with [`acquire_read`](Self::acquire_read) or [`acquire_write`](Self::acquire_write) before calling this.
    pub unsafe fn get(&self) -> *mut R {
        self.resource.get()
    }

    pub fn acquire_write(&self) -> EcsResult<()> {
        self.lock_marker.acquire_write()
    }

    pub fn release_write(&self) {
        self.lock_marker.release_write();
    }
}

trait ResourceHolder: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<R: Resource> ResourceHolder for ResourceSingleton<R> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[derive(Default)]
pub struct Resources {
    map: DashMap<TypeId, Box<dyn ResourceHolder>>
}

impl Resources {
    pub fn new() -> Self { Self::default() }

    pub fn insert<R: Resource>(&self, resource: R) {
        self.map.insert(TypeId::of::<R>(), Box::new(ResourceSingleton {
            resource: UnsafeCell::new(resource), lock_marker: LockMarker::new()
        }));
    }

    /// # Safety:
    /// 
    /// Aliasing invariants must be upheld manually.
    /// Ensure you've acauired a lock with [`acquire_read`](Self::acquire_read) before calling this.
    pub unsafe fn get<R: Resource>(&self) -> EcsResult<&R> {
        let singleton = self.map.get(&TypeId::of::<R>()).ok_or(EcsError::NotFound)?;
        let singleton: &ResourceSingleton<R> = singleton.as_any().downcast_ref().expect("Incorrect resource singleton inserted into map");

        Ok(unsafe {
            &*singleton.get()
        })
    }

    pub fn acquire_read<R: Resource>(&self) -> EcsResult<()> {
        let singleton = self.map.get(&TypeId::of::<R>()).ok_or(EcsError::NotFound)?;
        let singleton: &ResourceSingleton<R> = singleton.as_any().downcast_ref().expect("Incorrect resource singleton inserted into map");

        singleton.acquire_read()
    }

    pub fn release_read<R: Resource>(&self) {
        let Some(singleton) = self.map.get(&TypeId::of::<R>()) else { return };
        let singleton: &ResourceSingleton<R> = singleton.as_any().downcast_ref().expect("Incorrect resource singleton inserted into map");

        singleton.release_read();
    }

    /// # Safety:
    /// 
    /// Aliasing invariants must be upheld manually.
    /// Ensure you've acauired a lock with [`acquire_write`](Self::acquire_write) before calling this.
    pub unsafe fn get_mut<R: Resource>(&self) -> EcsResult<&mut R> {
        let mut singleton = self.map.get_mut(&TypeId::of::<R>()).ok_or(EcsError::NotFound)?;
        let singleton: &mut ResourceSingleton<R> = singleton.as_any_mut().downcast_mut().expect("Incorrect resource singleton inserted into map");

        Ok(unsafe {
            &mut *singleton.get()
        })
    }

    pub fn acquire_write<R: Resource>(&self) -> EcsResult<()> {
        let singleton = self.map.get(&TypeId::of::<R>()).ok_or(EcsError::NotFound)?;
        let singleton: &ResourceSingleton<R> = singleton.as_any().downcast_ref().expect("Incorrect resource singleton inserted into map");

        singleton.acquire_write()
    }

    pub fn release_write<R: Resource>(&self) {
        let Some(singleton) = self.map.get(&TypeId::of::<R>()) else { return };
        let singleton: &ResourceSingleton<R> = singleton.as_any().downcast_ref().expect("Incorrect resource singleton inserted into map");

        singleton.release_write()
    }
}

pub trait Resource: Send + Sync + 'static {}

pub struct Res<R: Resource> {
    pub(crate) locked: AtomicBool,
    pub(crate) world: Arc<World>,
    pub(crate) _marker: PhantomData<R>
}

impl<R: Resource> Deref for Res<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        let locked = self.locked.load(Ordering::SeqCst);
        if !locked {
            self.world.resources.acquire_read::<R>().unwrap();
            self.locked.store(true, Ordering::SeqCst);
        }

        unsafe {
            self.world.resources.get::<R>().unwrap()
        }
    }
}

impl<R: Resource> Drop for Res<R> {
    fn drop(&mut self) {
        if self.locked.load(Ordering::SeqCst) {
            self.world.resources.release_read::<R>();
        }
    }
}

pub struct ResMut<R: Resource> {
    pub(crate) locked: AtomicBool,
    pub(crate) world: Arc<World>,
    pub(crate) _marker: PhantomData<R>
}

impl<R: Resource> Deref for ResMut<R> {
    type Target = R;

    fn deref(&self) -> &Self::Target {
        let locked = self.locked.load(Ordering::SeqCst);
        if !locked {
            self.world.resources.acquire_write::<R>().unwrap();
            self.locked.store(true, Ordering::SeqCst);
        }

        unsafe {
            self.world.resources.get_mut::<R>().unwrap()
        }
    }
}

impl<R: Resource> DerefMut for ResMut<R> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let locked = self.locked.load(Ordering::SeqCst);
        if !locked {
            self.world.resources.acquire_write::<R>().unwrap();
            self.locked.store(true, Ordering::SeqCst);
        }

        unsafe {
            self.world.resources.get_mut::<R>().unwrap()
        }
    }
}

impl<R: Resource> Drop for ResMut<R> {
    fn drop(&mut self) {
        if self.locked.load(Ordering::SeqCst) {
            self.world.resources.release_write::<R>();
        }
    }
}