use log::info;
use crate::task::{
  suspend_current_and_run_next,
  exit_current_and_run_next,
  get_current_task_id,
  change_program_brk,
};
use crate::timer::get_time_ms;

pub fn sys_exit(xstate: i32) -> ! {
  info!("[kernel] Application {} exited with code {}", get_current_task_id(), xstate);
  exit_current_and_run_next()
}

pub fn sys_get_taskinfo() -> isize {
  get_current_task_id()
}

pub fn sys_yield() -> isize {
  suspend_current_and_run_next();
  0
}

pub fn sys_get_time() -> isize {
  get_time_ms() as isize
}

pub fn sys_sbrk(size: i32) -> isize {
  if let Some(old_brk) = change_program_brk(size) {
    return old_brk as isize
  } else {
    -1
  }
}
