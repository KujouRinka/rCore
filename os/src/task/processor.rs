use alloc::vec::Vec;
use alloc::sync::Arc;
use core::arch::asm;
use core::cell::UnsafeCell;
use core::ops::Index;
use lazy_static::lazy_static;
use crate::common::{cpuid, intr_get, intr_on, pop_off, push_off};
use crate::config::MAX_CPU_NUM;
use crate::task::{add_task, context::TaskContext, manager::fetch_task, switch::__switch, task::{TaskControlBlock, TaskStatus}};
use crate::trap::context::TrapContext;

pub struct Processors {
  processors: Vec<UnsafeCell<Processor>>,
}

unsafe impl Sync for Processors {}

impl Index<usize> for Processors {
  type Output = UnsafeCell<Processor>;

  fn index(&self, index: usize) -> &Self::Output {
    &self.processors[index]
  }
}

lazy_static! {
  pub static ref PROCESSOR: Processors = Processors {
    processors: {
      (0..MAX_CPU_NUM).map(|_| UnsafeCell::new(Processor::new())).collect()
    },
  };
}

pub struct Processor {
  current: Option<Arc<TaskControlBlock>>,
  scheduler_cx: TaskContext,
  pub noff: isize,
  pub intena: bool,
}

impl Processor {
  pub fn new() -> Self {
    Self {
      current: None,
      scheduler_cx: TaskContext::zero_init(),
      noff: 0,
      intena: false,
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

pub fn current_cpu() -> &'static mut Processor {
  unsafe { &mut *PROCESSOR[cpuid()].get() }
}

pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
  current_cpu().take_current()
}

pub fn current_task() -> Option<Arc<TaskControlBlock>> {
  push_off();
  let ret = current_cpu().current();
  pop_off();
  ret
}

#[allow(unused)]
pub fn current_user_token() -> Option<usize> {
  current_cpu()
    .current
    .as_ref()
    .map(|x| {
      x.inner_borrow_ptr().get_user_token()
    })
}

#[allow(unused)]
pub fn current_trap_cx() -> Option<&'static mut TrapContext> {
  current_task().map(|x| {
    x.inner_borrow_ptr().get_trap_cx()
  })
}

pub fn schedule() {
  if current_cpu().noff != 1 {
    panic!("sched locks");
  }
  if intr_get() {
    panic!("sched interruptible");
  }
  let intena = current_cpu().intena;
  let mut dummy = TaskContext::zero_init();
  let switched_task_cx_ptr = match take_current_task() {
    Some(task) => {
      let mut inner = task.inner_borrow_ptr_mut();
      inner.task_status = TaskStatus::Ready;
      let task_cx = &mut inner.task_cx as *mut TaskContext;
      add_task(task);
      task_cx
    }
    None => {
      &mut dummy as *mut TaskContext
    }
  };
  let processor = current_cpu();
  let this_cpu_scheduler_cx = processor.get_scheduler_cx_mut_ptr();
  unsafe {
    __switch(
      switched_task_cx_ptr,
      this_cpu_scheduler_cx,
    );
  }
  processor.intena = intena;
}

pub fn scheduler() {
  let processor = current_cpu();
  loop {
    intr_on();
    if let Some(next_task) = fetch_task() {
      next_task.lock();
      let pid = next_task.pid.0;
      let this_scheduler_cx = processor.get_scheduler_cx_mut_ptr();
      let mut next_task_inner = next_task.inner_borrow_ptr_mut();
      if next_task_inner.task_status != TaskStatus::Ready {
        next_task.unlock();
        continue;
      }
      let next_task_cx_ptr = &next_task_inner.task_cx as *const TaskContext;
      next_task_inner.task_status = TaskStatus::Running;

      let mu = next_task.get_mutex();
      processor.current = Some(next_task);
      unsafe {
        __switch(
          this_scheduler_cx,
          next_task_cx_ptr,
        );
      }
      processor.current = None;

      mu.unlock();
      if pid < 2 {
        intr_on();
        unsafe {
          asm!("wfi");
        }
      }
    }
  }
}
