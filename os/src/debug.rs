use alloc::string::String;
use log::debug;
use crate::common::cpuid;
use crate::task::current_task;

#[allow(unused)]
pub fn cpu_print(id: usize, s: String) {
  if id == cpuid() {
    debug!("{}", s);
  }
}

#[allow(unused)]
pub fn debug_current_task_id() -> isize {
  match current_task() {
    Some(task) => task.pid.0 as isize,
    None => -1,
  }
}
