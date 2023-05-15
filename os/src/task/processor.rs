use alloc::sync::Arc;
use lazy_static::lazy_static;
use crate::sync::UPSafeCell;
use crate::task::context::TaskContext;
use crate::task::manager::fetch_task;
use crate::task::switch::__switch;
use crate::task::task::{TaskControlBlock, TaskStatus};
use crate::trap::context::TrapContext;

lazy_static! {
  pub static ref PROCESSOR: UPSafeCell<Processor> = unsafe {
    UPSafeCell::new(Processor::new())
  };
}

pub struct Processor {
  current: Option<Arc<TaskControlBlock>>,
  scheduler_cx: TaskContext,
}

impl Processor {
  pub fn new() -> Self {
    Self {
      current: None,
      scheduler_cx: TaskContext::zero_init(),
    }
  }

  fn get_scheduler_cx_mut_ptr(&mut self) -> *mut TaskContext {
    &mut self.scheduler_cx as *mut _
  }

  pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
    self.current.take()
  }

  pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
    self.current.as_ref().map(Arc::clone)
  }
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
  PROCESSOR.exclusive_access().take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
  PROCESSOR.exclusive_access().current()
}

pub fn current_user_token() -> Option<usize> {
  PROCESSOR.exclusive_access()
    .current
    .as_ref()
    .map(|x| {
      x.inner_borrow().get_user_token()
    })
}

pub fn current_trap_cx() -> Option<&'static mut TrapContext> {
  current_task().map(|x| {
    x.inner_borrow().get_trap_cx()
  })
}

pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
  let this_cpu_scheduler_cx = &PROCESSOR.borrow().scheduler_cx as *const TaskContext;
  unsafe {
    __switch(
      switched_task_cx_ptr,
      this_cpu_scheduler_cx,
    );
  }
}

pub fn scheduler() {
  loop {
    let mut processor = PROCESSOR.exclusive_access();
    if let Some(next_task) = fetch_task() {
      let this_scheduler_cx = processor.get_scheduler_cx_mut_ptr();
      let mut next_task_inner = next_task.inner_exclusive_access();
      if next_task_inner.task_status != TaskStatus::Ready {
        continue;
      }
      let next_task_cx_ptr = &next_task_inner.task_cx as *const TaskContext;
      next_task_inner.task_status = TaskStatus::Running;

      drop(this_scheduler_cx);
      drop(next_task_inner);
      processor.current = Some(next_task);
      unsafe {
        __switch(
          this_scheduler_cx,
          next_task_cx_ptr,
        );
      }
    }
  }
}
