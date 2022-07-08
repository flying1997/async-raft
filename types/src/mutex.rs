use std::sync::Mutex as StdMutex;

pub use std::sync::MutexGuard;

/// A simple wrapper around the lock() function of a std::sync::Mutex
/// The only difference is that you don't need to call unwrap() on it.
#[derive(Debug)]
pub struct Mutex<T>(StdMutex<T>);

impl<T> Mutex<T> {
    /// creates mutex
    pub fn new(t: T) -> Self {
        Self(StdMutex::new(t))
    }

    /// lock the mutex
    pub fn lock(&self) -> MutexGuard<'_, T> {
        self.0
            .lock()
            .expect("RobotLink_v2 cannot currently handle a poisoned lock")
    }
}
