#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{
  get_taskinfo,
  get_time,
  MAX_SYSCALL_NUM,
  SyscallInfo,
  TaskInfo,
  TaskStatus,
  yield_,
};

#[no_mangle]
fn main() -> i32 {
  println!("Current time is {}", get_time());
  println!("Trying to get TaskInfo of 1");
  yield_();
  let mut task_info = TaskInfo {
    id: 0,
    status: TaskStatus::UnInit,
    call: [SyscallInfo { times: 0 }; MAX_SYSCALL_NUM],
    time: 0,
  };
  get_taskinfo(1, &mut task_info as *mut TaskInfo);
  println!("Task id: {}", task_info.id);
  println!("Status: {:?}", task_info.status);
  println!("Syscall info:");
  println!("\tsys_write: {}", task_info.call[64].times);
  println!("\tsys_exit: {}", task_info.call[93].times);
  println!("\tsys_yield: {}", task_info.call[124].times);
  println!("\tsys_get_time: {}", task_info.call[169].times);
  println!("\tsys_get_task_info: {}", task_info.call[410].times);
  println!("Time: {}", task_info.time);
  0
}
