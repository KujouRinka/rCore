use core::cell::UnsafeCell;
use core::fmt;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct SpinMutex<T: ?Sized> {
  /// false: unlocked
  /// true: locked
  futex: AtomicBool,
  data: UnsafeCell<T>,
}

unsafe impl<T: ?Sized + Send> Send for SpinMutex<T> {}

unsafe impl<T: ?Sized + Send> Sync for SpinMutex<T> {}

pub struct SpinMutexGuard<'a, T: ?Sized + 'a> {
  lock: &'a SpinMutex<T>,
}

impl<T: ?Sized> ! Send for SpinMutexGuard<'_, T> {}

unsafe impl<T: ?Sized + Sync> Sync for SpinMutexGuard<'_, T> {}

impl<T> SpinMutex<T> {
  pub const fn new(t: T) -> Self {
    Self {
      futex: AtomicBool::new(false),
      data: UnsafeCell::new(t),
    }
  }

  pub fn lock(&self) -> SpinMutexGuard<'_, T> {
    while let Err(_) = self.futex.compare_exchange(false, true, Acquire, Relaxed) {}
    unsafe {
      SpinMutexGuard::new(self)
    }
  }

  #[allow(unused)]
  pub fn unlock(guard: SpinMutexGuard<'_, T>) {
    drop(guard);
  }

  #[allow(unused)]
  pub fn try_lock(&self) {
    unimplemented!()
  }
}

impl<'mutex, T: ?Sized> SpinMutexGuard<'mutex, T> {
  unsafe fn new(lock: &'mutex SpinMutex<T>) -> SpinMutexGuard<'mutex, T> {
    Self { lock }
  }
}

impl<T: ?Sized> Deref for SpinMutexGuard<'_, T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { &*self.lock.data.get() }
  }
}

impl<T: ?Sized> DerefMut for SpinMutexGuard<'_, T> {
  fn deref_mut(&mut self) -> &mut Self::Target {
    unsafe { &mut *self.lock.data.get() }
  }
}

impl<T: ?Sized> Drop for SpinMutexGuard<'_, T> {
  fn drop(&mut self) {
    if self.lock.futex.swap(false, Release) == false {
      panic!("SpinMutexGuard was not locked");
    }
  }
}

impl<T: ?Sized + fmt::Debug> fmt::Debug for SpinMutexGuard<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    fmt::Debug::fmt(&**self, f)
  }
}

impl<T: ?Sized + fmt::Display> fmt::Display for SpinMutexGuard<'_, T> {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    (**self).fmt(f)
  }
}
