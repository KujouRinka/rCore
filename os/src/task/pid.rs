use alloc::vec::Vec;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use crate::config::*;
use crate::mm::{KERNEL_SPACE, MapPermission};

pub struct PidHandle(pub usize);

impl Drop for PidHandle {
  fn drop(&mut self) {
    PID_ALLOCATOR.exclusive_access().dealloc(self.0);
  }
}

struct PidAllocator {
  current: usize,
  recycled: Vec<usize>,
}

impl PidAllocator {
  pub fn new() -> Self {
    Self {
      current: 0,
      recycled: Vec::new(),
    }
  }

  pub fn alloc(&mut self) -> PidHandle {
    if let Some(pid) = self.recycled.pop() {
      PidHandle(pid)
    } else {
      let ret = PidHandle(self.current);
      self.current += 1;
      ret
    }
  }

  pub fn dealloc(&mut self, pid: usize) {
    assert!(pid < self.current);
    assert!(
      self.recycled.iter().find(|ppid| **ppid == pid).is_none(),
      "pid {} has been deallocated but should not!", pid
    );
    self.recycled.push(pid);
  }
}

lazy_static! {
  static ref PID_ALLOCATOR: UPSafeCell<PidAllocator> = unsafe {
    UPSafeCell::new(PidAllocator::new())
  };
}

pub fn pid_alloc() -> PidHandle {
  PID_ALLOCATOR.exclusive_access().alloc()
}

pub struct KernelStack {
  pid: usize,
}

impl KernelStack {
  pub fn new(pid_handle: &PidHandle) -> Self {
    let pid = pid_handle.0;
    let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(pid);
    unsafe {
      KERNEL_SPACE.exclusive_access()
        .insert_framed_area(
          kernel_stack_bottom.into(),
          kernel_stack_top.into(),
          MapPermission::R | MapPermission::W,
        );
    }
    Self { pid }
  }

  pub fn push_on_top<T>(&self, value: T) -> *mut T
    where T: Sized
  {
    let kernel_stack_top = self.get_top();
    let ptr_mut = (kernel_stack_top - core::mem::size_of::<T>()) as *mut T;
    unsafe { *ptr_mut = value; }
    ptr_mut
  }

  pub fn get_top(&self) -> usize {
    kernel_stack_position(self.pid).1
  }
}

impl Drop for KernelStack {
  fn drop(&mut self) {
    let (kernel_stack_bottom, _) = kernel_stack_position(self.pid);
    KERNEL_SPACE.exclusive_access()
      .remove_area_with_start_vpn(kernel_stack_bottom.into());
  }
}

/// # Layout
/// ```
/// +-------------------+
/// |    Trampoline     |
/// |-------------------|
/// |    Guard Page     |
/// |-------------------|
/// |  Kernel Stack 0   |
/// |-------------------|
/// |    Guard Page     |
/// |-------------------|
/// |  Kernel Stack 1   |
/// |-------------------|
/// |        ...        |
/// |                   |
/// ```
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
  let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
  let bottom = top - KERNEL_STACK_SIZE;
  (bottom, top)
}
