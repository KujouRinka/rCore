use core::fmt;
use core::cell::{Cell, UnsafeCell};
use core::ops::{Deref, DerefMut};
use core::sync::atomic::AtomicBool;
use core::sync::atomic::Ordering::{Acquire, Relaxed, Release};
use crate::common::{pop_off, push_off, r_tp};

pub struct SpinLock(AtomicBool, Cell<isize>);

unsafe impl Send for SpinLock {}

unsafe impl Sync for SpinLock {}

impl SpinLock {
  pub fn new() -> Self {
    Self(AtomicBool::new(false), Cell::new(-1))
  }

  pub fn lock(&self) {
    push_off();
    if self.holding() {
      panic!("SpinLock was locked by current thread");
    }
    while let Err(_) = self.0.compare_exchange(false, true, Acquire, Relaxed) {}
    self.1.set(r_tp() as isize);
  }

  pub fn unlock(&self) {
    if !self.holding() {
      panic!("SpinLock was not locked");
    }
    self.1.set(-1);
    if self.0.swap(false, Release) == false {
      panic!("SpinLock was not locked");
    }
    pop_off();
  }

  fn holding(&self) -> bool {
    self.0.load(Relaxed) && self.1.get() == r_tp() as isize
  }
}

pub struct SpinMutex<T: ?Sized> {
  /// false: unlocked
  /// true: locked
  futex: AtomicBool,
  cpu: Cell<isize>,
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
      cpu: Cell::new(-1),
      data: UnsafeCell::new(t),
    }
  }

  pub fn lock(&self) -> SpinMutexGuard<'_, T> {
    push_off();
    if self.holding() {
      panic!("SpinLock was locked by current thread");
    }
    while let Err(_) = self.futex.compare_exchange(false, true, Acquire, Relaxed) {}
    self.cpu.set(r_tp() as isize);
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

  fn holding(&self) -> bool {
    self.futex.load(Relaxed) && self.cpu.get() == r_tp() as isize
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
    self.lock.cpu.set(-1);
    if self.lock.futex.swap(false, Release) == false {
      panic!("SpinMutexGuard was not locked");
    }
    pop_off();
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
