mod fs;
mod process;

use log::error;
use fs::*;
use process::*;
use crate::task::exit_current_and_run_next;

const SYSCALL_WRITE: usize = 64;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_YIELD: usize = 124;
const SYSCALL_GET_TASKINFO: usize = 114514;

// TODO: performance: may replace with a syscall table
//  `match` slows down function select

pub fn syscall(which: usize, args: [usize; 3]) -> isize {
  match which {
    SYSCALL_WRITE => sys_write(args[0], args[1] as *const u8, args[2]),
    SYSCALL_EXIT => sys_exit(args[0] as i32),
    SYSCALL_YIELD => sys_yield(),
    SYSCALL_GET_TASKINFO => sys_get_taskinfo(),
    _ => {
      error!("Unsupported syscall: {}", which);
      exit_current_and_run_next()
    },
  }
}
