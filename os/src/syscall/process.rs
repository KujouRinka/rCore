use log::warn;
use crate::println;
use crate::task::{
  suspend_current_and_run_next,
  exit_current_and_run_next,
  get_current_task_id,
  task::TaskInfo,
  TASK_MANAGER,
};
use crate::timer::{get_time, get_time_ms};

pub fn sys_exit(xstate: i32) -> ! {
  println!("[kernel] Application exited with code {}", xstate);
  warn!("Application {} exited with code {}", get_current_task_id(), xstate);
  exit_current_and_run_next()
}

pub fn sys_task_info(id: usize, ts: *mut TaskInfo) -> isize {
  if id >= TASK_MANAGER.num_app {
    return -1;
  }
  let ts = unsafe { &mut *ts };
  ts.id = id;
  let inner = TASK_MANAGER.inner.exclusive_access();
  let current_task = inner.current_task;
  ts.status = inner.tasks[current_task].task_status;
  ts.call = inner.tasks[current_task].call;
  ts.time = get_time() - inner.tasks[current_task].start_time;
  0
}

pub fn sys_yield() -> isize {
  suspend_current_and_run_next();
  0
}

pub fn sys_get_time() -> isize {
  get_time_ms() as isize
}
