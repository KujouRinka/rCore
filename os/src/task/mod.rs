use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::{trace, info};
use crate::sync::UPSafeCell;
use crate::task::task::{TaskControlBlock, TaskStatus};
use crate::loader::{get_app_data, get_num_app};
use crate::sbi::shutdown;
use crate::task::context::TaskContext;
use switch::__switch;
use crate::trap::context::TrapContext;

mod switch;
mod context;
mod task;

pub struct TaskManager {
  num_app: usize,
  inner: UPSafeCell<TaskManagerInner>,
}

impl TaskManager {
  fn run_first_task(&self) -> ! {
    let mut inner = self.inner.exclusive_access();
    inner.current_task = 0;
    let task0 = &mut inner.tasks[0];
    task0.task_status = TaskStatus::Running;
    let next_task_cx_ptr = &task0.task_cx as *const TaskContext;
    drop(inner);
    let mut dummy = TaskContext::zero_init();
    unsafe {
      __switch(
        &mut dummy as *mut TaskContext,
        next_task_cx_ptr,
      );
    }
    panic!("Unreachable in run_first_task!")
  }

  fn mark_current_suspended(&self) {
    let mut inner = self.inner.exclusive_access();
    let current_task_id = inner.current_task;
    inner.tasks[current_task_id].task_status = TaskStatus::Ready;
  }

  fn mark_current_exited(&self) {
    let mut inner = self.inner.exclusive_access();
    let current_task_id = inner.current_task;
    inner.tasks[current_task_id].task_status = TaskStatus::Exited;
  }

  fn run_next_task(&self) {
    if let Some(next) = self.find_next_task() {
      let mut inner = self.inner.exclusive_access();
      let current_id = inner.current_task;
      inner.tasks[next].task_status = TaskStatus::Running;
      inner.current_task = next;
      let current_task_cx_ptr = &mut inner.tasks[current_id].task_cx as *mut TaskContext;
      let next_task_cx_ptr = &inner.tasks[next].task_cx as *const TaskContext;
      drop(inner);
      unsafe {
        __switch(current_task_cx_ptr, next_task_cx_ptr);
      }
    } else {
      info!("All applications completed!");
      shutdown()
    }
  }

  fn find_next_task(&self) -> Option<usize> {
    let inner = self.inner.exclusive_access();
    let current = inner.current_task;
    (current + 1..current + self.num_app + 1)
      .map(|id| id % self.num_app)
      .find(|id| inner.tasks[*id].task_status == TaskStatus::Ready)
  }

  fn change_current_program_brk(&self, size: i32) -> Option<usize> {
    let mut inner = self.inner.exclusive_access();
    let current = inner.current_task;
    inner.tasks[current].change_brk(size)
  }
}

impl TaskManager {
  /// Current token for satp register.
  fn get_current_token(&self) -> usize {
    let inner = self.inner.exclusive_access();
    let current = inner.current_task;
    inner.tasks[current].get_user_token()
  }

  /// Current `TrapContext` pointer.
  fn get_current_trap_cx(&self) -> &'static mut TrapContext {
    let inner = self.inner.exclusive_access();
    let current = inner.current_task;
    inner.tasks[current].get_trap_cx()
  }
}

struct TaskManagerInner {
  tasks: Vec<TaskControlBlock>,
  current_task: usize,
}

lazy_static! {
  pub static ref TASK_MANAGER: TaskManager = {
    trace!("init TASK_MANAGER");
    let num_app = get_num_app();
    trace!("num_app = {}", num_app);
    let mut tasks = Vec::with_capacity(num_app);
    for i in 0..num_app {
      tasks.push(TaskControlBlock::new(
        get_app_data(i),
        i,
      ))
    }
    TaskManager {
      num_app,
      inner: unsafe {
        UPSafeCell::new(TaskManagerInner {
          tasks,
          current_task: 0,
        })
      },
    }
  };
}

// This is same as yield()
pub fn suspend_current_and_run_next() {
  mark_current_suspended();
  run_next_task();
}

pub fn exit_current_and_run_next() -> ! {
  mark_current_exited();
  run_next_task();
  panic!("Unreachable in exit_current_and_run_next")
}

pub fn run_first_task() {
  TASK_MANAGER.run_first_task();
}

fn mark_current_suspended() {
  TASK_MANAGER.mark_current_suspended();
}

fn mark_current_exited() {
  TASK_MANAGER.mark_current_exited();
}

fn run_next_task() {
  TASK_MANAGER.run_next_task();
}

pub fn get_current_task_id() -> isize {
  let inner = TASK_MANAGER.inner.exclusive_access();
  inner.current_task as isize
}

pub fn get_current_token() -> usize {
  TASK_MANAGER.get_current_token()
}

pub fn get_current_trap_cx() -> &'static mut TrapContext {
  TASK_MANAGER.get_current_trap_cx()
}

pub fn change_program_brk(size: i32) -> Option<usize> {
  TASK_MANAGER.change_current_program_brk(size)
}
