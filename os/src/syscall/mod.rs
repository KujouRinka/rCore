mod fs;
pub(crate) mod process;

use log::error;
use fs::*;
use process::*;
use crate::task::{exit_current_and_run_next, TASK_MANAGER};
use crate::task::task::TaskInfo;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TIME: usize = 169;
const SYSCALL_GET_TASK_INFO: usize = 410;

// NODE: the max syscall number is 411
//  because the syscall number is stored in a byte
//  too large syscall number will cause kernel stack overflow.
pub const MAX_SYSCALL_NUM: usize = 411;

#[derive(Copy, Clone)]
pub struct SyscallInfo {
  times: usize,
}

// TODO: performance: may replace with a syscall table
//  `match` slows down function select

pub fn syscall(which: usize, args: [usize; 3]) -> isize {
  let mut task_manager_inner = TASK_MANAGER.inner.exclusive_access();
  let current_task = task_manager_inner.current_task;
  match task_manager_inner.tasks[current_task].call.get_mut(which) {
    Some(syscall_info) => syscall_info.times += 1,
    None => {}
  }
  drop(task_manager_inner);
  match which {
    SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
    SYSCALL_EXIT => sys_exit(args[0] as i32),
    SYSCALL_YIELD => sys_yield(),
    SYSCALL_GET_TIME => sys_get_time(),
    SYSCALL_GET_TASK_INFO => sys_task_info(args[0], args[1] as *mut TaskInfo),
    _ => {
      error!("Unsupported syscall: {}", which);
      exit_current_and_run_next()
    }
  }
}

impl SyscallInfo {
  pub fn new() -> Self {
    SyscallInfo {
      times: 0,
    }
  }
}
