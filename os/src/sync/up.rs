// Wrap RefCell to be Sync

use core::cell::{Ref, RefCell, RefMut};

pub struct UPSafeCell<T> {
  inner: RefCell<T>,
}

unsafe impl<T> Sync for UPSafeCell<T> {}

#[allow(unused)]
impl<T> UPSafeCell<T> {
  pub unsafe fn new(value: T) -> Self {
    Self {
      inner: RefCell::new(value),
    }
  }

  #[allow(unused)]
  pub fn borrow_mut(&self) -> RefMut<'_, T> {
    self.inner.borrow_mut()
  }

  #[allow(unused)]
  pub fn borrow(&self) -> Ref<'_, T> {
    self.inner.borrow()
  }

  pub fn borrow_ptr_mut(&self) -> &'static mut T {
    unsafe { &mut *self.inner.as_ptr() }
  }

  pub fn borrow_ptr(&self) -> &'static T {
    unsafe { &*self.inner.as_ptr() }
  }
}
