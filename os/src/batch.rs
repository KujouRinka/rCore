use core::arch::asm;
use lazy_static::lazy_static;
use log::warn;
use crate::println;
use crate::sbi::shutdown;
use crate::sync::UPSafeCell;
use crate::trap::context::TrapContext;

const USER_STACK_SIZE: usize = 1 << 13;
const KERNEL_STACK_SIZE: usize = 1 << 13;
const MAX_APP_NUM: usize = 16;
const APP_BASE_ADDRESS: usize = 0x80400000;
const APP_SIZE_LIMIT: usize = 0x20000;

#[repr(align(4096))]
struct KernelStack {
  data: [u8; KERNEL_STACK_SIZE],
}

#[repr(align(4096))]
struct UserStack {
  data: [u8; USER_STACK_SIZE],
}

static KERNEL_STACK: KernelStack = KernelStack { data: [0; KERNEL_STACK_SIZE] };
static USER_STACK: UserStack = UserStack { data: [0; USER_STACK_SIZE] };

impl KernelStack {
  fn get_sp(&self) -> usize {
    self.data.as_ptr() as usize + KERNEL_STACK_SIZE
  }

  pub fn push_context(&self, cx: TrapContext) -> &'static mut TrapContext {
    let cx_ptr = (self.get_sp() - core::mem::size_of::<TrapContext>()) as *mut TrapContext;
    unsafe {
      core::ptr::write(cx_ptr, cx);
      cx_ptr.as_mut().unwrap()
    }
  }
}

impl UserStack {
  fn get_sp(&self) -> usize {
    self.data.as_ptr() as usize + USER_STACK_SIZE
  }
}

struct AppManager {
  num_app: usize,
  current_app: usize,
  app_start: [usize; MAX_APP_NUM + 1],
}

impl AppManager {
  pub fn print_app_info(&self) {
    println!("[kernel] num_app = {}", self.num_app);
    for i in 0..self.num_app {
      println!(
        "[kernel] app_{} [{:#x}, {:#x})",
        i,
        self.app_start[i],
        self.app_start[i + 1]
      );
    }
  }

  pub fn get_current_app(&self) -> usize {
    self.current_app
  }

  pub fn move_to_next_app(&mut self) {
    self.current_app += 1;
  }

  /// Load specified app to `APP_BASE_ADDRESS`
  unsafe fn load_app(&self, app_id: usize) {
    if app_id >= self.num_app {
      warn!("All applications completed!");
      shutdown();
    }
    println!("[kernel] Loading app_{}", app_id);

    core::slice::from_raw_parts_mut(
      APP_BASE_ADDRESS as *mut u8,
      APP_SIZE_LIMIT,
    ).fill(0);

    let src = core::slice::from_raw_parts(
      self.app_start[app_id] as *const u8,
      self.app_start[app_id + 1] - self.app_start[app_id],
    );

    let dst = core::slice::from_raw_parts_mut(
      APP_BASE_ADDRESS as *mut u8,
      src.len(),
    );

    dst.copy_from_slice(src);
    asm!("fence.i");
  }
}



lazy_static! {
  static ref APP_MANAGER: UPSafeCell<AppManager> = unsafe {
    UPSafeCell::new({
      extern "C" {
        fn _num_app();
      }
      let p = _num_app as *const usize;
      let num_app = p.read_volatile();
      let mut app_start = [0usize; MAX_APP_NUM + 1];
      let in_mem_slice = core::slice::from_raw_parts(
        p.add(1), num_app + 1
      );
      app_start[..=num_app].copy_from_slice(in_mem_slice);
      AppManager{
        num_app,
        current_app: 0,
        app_start,
      }
    })
  };
}

pub fn init() {
  print_app_info();
}

pub fn print_app_info() {
  APP_MANAGER.exclusive_access().print_app_info();
}

pub fn run_next_app() -> ! {
  let mut manager = APP_MANAGER.exclusive_access();
  let current_app = manager.get_current_app();
  unsafe {
    manager.load_app(current_app);
  }
  manager.move_to_next_app();
  drop(manager);
  extern "C" { fn __restore(cx_addr: usize); }
  unsafe {
    let new_trap_ctx = TrapContext::app_init_context(
      APP_BASE_ADDRESS,
      USER_STACK.get_sp(),
    );
    __restore(
      KERNEL_STACK.push_context(new_trap_ctx) as *const _ as usize
    );
  }
  panic!("Unreachable in batch::run_current_app!")
}
