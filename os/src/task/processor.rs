use alloc::vec::Vec;
use alloc::sync::Arc;
use core::cell::RefCell;
use core::ops::Index;
use lazy_static::lazy_static;
use crate::common::cpuid;
use crate::config::MAX_CPU_NUM;
use crate::task::{
  context::TaskContext,
  manager::fetch_task,
  switch::__switch,
  task::{TaskControlBlock, TaskStatus},
};
use crate::trap::context::TrapContext;

pub struct Processors {
  processors: Vec<RefCell<Processor>>,
}

unsafe impl Sync for Processors {}

impl Index<usize> for Processors {
  type Output = RefCell<Processor>;

  fn index(&self, index: usize) -> &Self::Output {
    &self.processors[index]
  }
}

lazy_static! {
  pub static ref PROCESSOR: Processors = Processors {
    processors: {
      (0..MAX_CPU_NUM).map(|_| RefCell::new(Processor::new())).collect()
    },
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
  PROCESSOR[cpuid()].borrow_mut().take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
  PROCESSOR[cpuid()].borrow().current()
}

#[allow(unused)]
pub fn current_user_token() -> Option<usize> {
  PROCESSOR[cpuid()].borrow()
    .current
    .as_ref()
    .map(|x| {
      x.inner_borrow().get_user_token()
    })
}

#[allow(unused)]
pub fn current_trap_cx() -> Option<&'static mut TrapContext> {
  current_task().map(|x| {
    x.inner_borrow().get_trap_cx()
  })
}

pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
  let mut processor = PROCESSOR[cpuid()].borrow_mut();
  let this_cpu_scheduler_cx = processor.get_scheduler_cx_mut_ptr();
  drop(processor);
  unsafe {
    __switch(
      switched_task_cx_ptr,
      this_cpu_scheduler_cx,
    );
  }
}

pub fn scheduler() {
  loop {
    let mut processor = PROCESSOR[cpuid()].borrow_mut();
    if let Some(next_task) = fetch_task() {
      let this_scheduler_cx = processor.get_scheduler_cx_mut_ptr();
      let mut next_task_inner = next_task.inner_borrow_mut();
      if next_task_inner.task_status != TaskStatus::Ready {
        continue;
      }
      let next_task_cx_ptr = &next_task_inner.task_cx as *const TaskContext;
      next_task_inner.task_status = TaskStatus::Running;

      drop(next_task_inner);
      processor.current = Some(next_task);
      drop(processor);
      unsafe {
        __switch(
          this_scheduler_cx,
          next_task_cx_ptr,
        );
      }
    }
  }
}
