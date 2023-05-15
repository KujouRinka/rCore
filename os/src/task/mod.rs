use alloc::sync::Arc;
use lazy_static::lazy_static;
use crate::loader::{get_app_data_by_name, list_apps};
use crate::task::task::{TaskControlBlock, TaskStatus};
use crate::task::context::TaskContext;
use crate::task::processor::{current_task, schedule};
pub(crate) use crate::task::manager::add_task;
#[cfg(feature = "sbrk_lazy_alloc")]
use crate::mm::VirtAddr;
use crate::trap::context::TrapContext;
pub(crate) use crate::task::processor::scheduler;

mod switch;
mod context;
mod task;
mod pid;
mod manager;
mod processor;

pub fn init() {
  add_initproc();
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
pub fn suspend_current_and_run_next() {
  let task = get_current_task();
  let mut inner = task.inner_exclusive_access();
  inner.task_status = TaskStatus::Ready;
  let task_cx = &mut inner.task_cx as *mut TaskContext;

  drop(inner);
  add_task(task);
  schedule(task_cx);
}

pub fn exit_current_and_run_next(xcode: i32) -> ! {
  let task = get_current_task();
  let mut task_inner = task.inner_exclusive_access();
  task_inner.task_status = TaskStatus::Zombie;
  task_inner.xcode = xcode;

  let mut initproc_inner = INITPROC.inner_exclusive_access();
  for child in task_inner.children.iter() {
    child.inner_exclusive_access().parent = Some(Arc::downgrade(&INITPROC));
    initproc_inner.children.push(Arc::clone(child));
  }
  drop(initproc_inner);

  // Manually call this to free all pages
  unsafe {
    task_inner.memory_set.recycle_pages();
  }
  drop(task_inner);
  drop(task);

  let mut dummy = TaskContext::zero_init();
  schedule(&mut dummy as *mut _);

  panic!("Unreachable in exit_current_and_run_next")
}

pub fn get_current_pid() -> isize {
  get_current_task().get_pid() as isize
}

pub fn get_current_token() -> usize {
  get_current_task().inner_borrow().get_user_token()
}

pub fn get_current_trap_cx() -> &'static mut TrapContext {
  get_current_task().inner_borrow().get_trap_cx()
}

pub fn get_current_tcb_ref() -> &'static TaskControlBlock {
  unsafe { core::mem::transmute(get_current_task().as_ref()) }
}

pub fn change_program_brk(size: i32) -> Option<usize> {
  if let Some(mut task) = current_task() {
    task.inner_exclusive_access().change_brk(size)
  } else {
    None
  }
}

#[cfg(feature = "sbrk_lazy_alloc")]
pub fn lazy_alloc_page(addr: VirtAddr) -> bool {
  if let Some(mut task) = current_task() {
    task.inner_exclusive_access().lazy_alloc_page(addr)
  } else {
    false
  }
}
