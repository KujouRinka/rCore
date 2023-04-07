use crate::batch::{get_taskid, run_next_app};
use crate::println;

pub fn sys_exit(xstate: i32) -> ! {
  println!("[kernel] Application exited with code {}", xstate);
  run_next_app()
}

pub fn sys_get_taskinfo() -> isize {
  get_taskid() as isize - 1
}
