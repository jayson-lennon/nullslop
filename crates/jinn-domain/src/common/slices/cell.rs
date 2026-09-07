//! TypedCell — a shared, per-slice storage cell.
//!
//! A [`TypedCell`] is the write handle minted by [`Slices::register`]
//! (see [`super`]). It wraps an `Arc<RwLock<T>>` so every clone — owner,
//! renderer, or router — observes the same payload, while mutation stays
//! closure-scoped: no `&mut T` ever escapes, so a render thread can never
//! observe a half-applied update.

use std::sync::Arc;

use parking_lot::{RwLock, RwLockReadGuard};

/// A typed handle to one render slice's payload.
///
/// There is deliberately no way to obtain `&mut T` from outside this
/// module's constructor call chain: owners mutate through
/// [`update`](Self::update), readers snapshot through
/// [`read`](Self::read). Holding a guard across an `.await` is the one
/// misuse the type cannot prevent — treat it as forbidden.
#[derive(derive_more::Debug)]
pub struct TypedCell<T>
where
    T: Send + Sync + 'static,
{
    #[debug("TypedCell<{}>", std::any::type_name::<T>())]
    inner: Arc<RwLock<T>>,
}

impl<T> Clone for TypedCell<T>
where
    T: Send + Sync + 'static,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<T> TypedCell<T>
where
    T: Send + Sync + 'static,
{
    pub(super) fn new(value: T) -> Self {
        Self {
            inner: Arc::new(RwLock::new(value)),
        }
    }

    /// Applies `f` to the payload under the write lock.
    ///
    /// This is the only mutation path. The closure runs to completion
    /// before the lock is released, so each `update` is atomic to readers.
    pub fn update<F>(&self, f: F)
    where
        F: FnOnce(&mut T),
    {
        let mut guard = self.inner.write();
        f(&mut guard);
    }

    /// Snapshots the payload for reading.
    ///
    /// The guard borrows the cell's lock: drop it before awaiting.
    pub fn read(&self) -> RwLockReadGuard<'_, T> {
        self.inner.read()
    }
}
