mod switch;
mod context;
mod task;
mod pid;
mod manager;
mod processor;

use alloc::sync::Arc;
use lazy_static::lazy_static;
use log::info;

use task::{TaskControlBlock, TaskStatus};
use processor::{schedule, take_current_task};
pub(crate) use manager::add_task;
pub(crate) use processor::{current_task, scheduler, current_cpu};

use crate::loader::{get_app_data_by_name, list_apps};
#[cfg(feature = "sbrk_lazy_alloc")]
use crate::mm::VirtAddr;
use crate::sbi::shutdown;
use crate::trap::context::TrapContext;

pub fn init() {
  add_initproc();
  info!("initproc created and loaded");
  list_apps();
}

lazy_static! {
  pub static ref INITPROC: Arc<TaskControlBlock> = Arc::new(
    TaskControlBlock::new_for_initproc(get_app_data_by_name("initproc").unwrap())
  );
}

pub fn add_initproc() {
  add_task(INITPROC.clone());
}

// ----------
// helper function below

pub fn get_current_task() -> Arc<TaskControlBlock> {
  if let Some(task) = current_task() {
    task
  } else {
    panic!("An application must running but not!");
  }
}

// This is same as yield()
pub fn yield_() {
  let task = get_current_task();
  task.lock();
  let mu = task.get_mutex();
  schedule();
  mu.unlock();
}

pub const INITPROC_PID: usize = 0;

pub fn exit(xcode: i32) -> ! {
  let task = take_current_task().unwrap();
  let pid = task.get_pid();
  if pid == INITPROC_PID {
    shutdown();
  }
  let mut task_inner = task.inner_borrow_ptr_mut();


  INITPROC.lock();
  task.lock();

  let initproc_inner = INITPROC.inner_borrow_ptr_mut();
  for child in task_inner.children.iter() {
    child.lock();
    let child_inner = child.inner_borrow_ptr_mut();
    child_inner.parent = Some(Arc::downgrade(&INITPROC));
    initproc_inner.children.push(Arc::clone(child));
    child.unlock();
  }

  task_inner.task_status = TaskStatus::Zombie;
  task_inner.xcode = xcode;
  // Must drop all ref to children manually.
  task_inner.children.clear();
  // Manually call this to free all pages
  unsafe {
    task_inner.memory_set.recycle_pages();
  }

  drop(task);
  INITPROC.unlock();

  schedule();

  panic!("Unreachable in exit()")
}

pub fn get_current_pid() -> isize {
  get_current_task().get_pid() as isize
}

pub fn get_current_token() -> usize {
  get_current_task().inner_borrow_ptr().get_user_token()
}

pub fn get_current_trap_cx() -> &'static mut TrapContext {
  get_current_task().inner_borrow_ptr().get_trap_cx()
}

pub fn get_current_tcb_ref() -> &'static TaskControlBlock {
  unsafe { core::mem::transmute(get_current_task().as_ref()) }
}

pub fn change_program_brk(size: i32) -> Option<usize> {
  if let Some(task) = current_task() {
    task.change_brk(size)
  } else {
    None
  }
}

#[cfg(feature = "sbrk_lazy_alloc")]
pub fn lazy_alloc_page(addr: VirtAddr) -> bool {
  if let Some(task) = current_task() {
    task.lazy_alloc_page(addr)
  } else {
    false
  }
}
