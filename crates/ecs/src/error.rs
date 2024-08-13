use thiserror::Error;

#[derive(Debug, Error, Copy, Clone, PartialEq, Eq)]
pub enum EcsError {
    /// The modification was rejected because the storage is currently in use.
    #[error("the operation was rejected because the requested component storage is already locked: {0}")]
    StorageLocked(&'static str)
}   

pub type EcsResult<T> = Result<T, EcsError>;