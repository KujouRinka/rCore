use core::arch::asm;
use crate::trap::context::TrapContext;
use crate::config::*;

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct KernelStack {
  data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
#[derive(Copy, Clone)]
struct UserStack {
  data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: [KernelStack; MAX_APP_NUM] = [
  KernelStack { data: [0; KERNEL_STACK_SIZE] };
  MAX_APP_NUM
];

static USER_STACK: [UserStack; MAX_APP_NUM] = [
  UserStack { data: [0; USER_STACK_SIZE] };
  MAX_APP_NUM
];

impl KernelStack {
  fn get_sp(&self) -> usize {
    self.data.as_ptr() as usize + KERNEL_STACK_SIZE
  }

  pub fn push_context(&self, cx: TrapContext) -> usize {
    let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
    unsafe {
      *cx_ptr = cx;
    }
    cx_ptr as usize
  }
}

impl UserStack {
  fn get_sp(&self) -> usize {
    self.data.as_ptr() as usize + USER_STACK_SIZE
  }
}

fn get_base_i(app_id: usize) -> usize {
  APP_BASE_ADDRESS + APP_SIZE_LIMIT * app_id
}

pub fn get_num_app() -> usize {
  extern "C" {
    fn _num_app();
  }
  unsafe { (_num_app as *const usize).read_volatile() }
}

pub fn load_apps() {
  extern "C" { fn _num_app(); }
  let num_app_ptr = _num_app as *const usize;
  let num_app = get_num_app();
  let app_start = unsafe {
    core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1)
  };
  for i in 0..num_app {
    let base_i = get_base_i(i);
    unsafe {
      core::ptr::write_bytes(
        base_i as *mut u8,
        0,
        APP_SIZE_LIMIT,
      );
    }
    // Copy app bytes from src to dst
    unsafe {
      core::ptr::copy(
        app_start[i] as *const u8,
        base_i as *mut u8,
        app_start[i + 1] - app_start[i],
      );
    }
  }
  unsafe {
    asm!("fence.i");
  }
}

/// Set app entry and user stack for first run.
pub fn init_app_cx(app_id: usize) -> usize {
  KERNEL_STACK[app_id].push_context(TrapContext::app_init_context(
    get_base_i(app_id),
    USER_STACK[app_id].get_sp(),
  ))
}
