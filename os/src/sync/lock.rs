use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};

pub struct Spinlock {}

pub struct SpinMutex<T: ?Sized> {
  /// 0: unlocked
  /// 1: locked, no other threads waiting
  /// 2: locked, and other threads waiting (contended)
  futex: AtomicU32,
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
      futex: AtomicU32::new(0),
      data: UnsafeCell::new(t),
    }
  }

  pub fn lock(&self) -> SpinMutexGuard<'_, T> {
    while let Ok(_) = self.futex.compare_exchange(0, 1, Acquire, Relaxed) {}
    unsafe {
      SpinMutexGuard::new(self)
    }
  }

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
    if self.lock.futex.swap(0, Release) == 0 {
      panic!("SpinMutexGuard was not locked");
    }
  }
}
