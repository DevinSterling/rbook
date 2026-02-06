#[cfg(feature = "threadsafe")]
pub(crate) mod inner {
    use std::sync::{LockResult, Mutex, MutexGuard};

    /// Marker to identify an implementing instance as thread-safe.
    pub trait SendAndSync: Send + Sync {}

    pub(crate) struct Lock<T>(Mutex<T>);

    impl<T> Lock<T> {
        pub(crate) fn new(t: T) -> Self {
            Self(Mutex::new(t))
        }

        pub(crate) fn lock(&self) -> LockResult<MutexGuard<'_, T>> {
            self.0.lock()
        }
    }
}

#[cfg(not(feature = "threadsafe"))]
pub(crate) mod inner {
    use std::cell::{RefCell, RefMut};

    pub trait SendAndSync {}

    pub(crate) struct Lock<T>(RefCell<T>);

    impl<T> Lock<T> {
        pub(crate) fn new(t: T) -> Self {
            Self(RefCell::new(t))
        }

        pub(crate) fn lock(&self) -> std::sync::LockResult<RefMut<'_, T>> {
            Ok(self.0.borrow_mut())
        }
    }
}

pub(crate) use inner::{Lock, SendAndSync};

impl<#[cfg(feature = "threadsafe")] A: Send + Sync, #[cfg(not(feature = "threadsafe"))] A>
    SendAndSync for A
{
}
