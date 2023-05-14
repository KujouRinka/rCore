use log::info;
use crate::loader::get_app_data_by_name;
use crate::mm::translate_str;
use crate::task::{
  get_current_task,
  suspend_current_and_run_next,
  exit_current_and_run_next,
  get_current_pid,
  get_current_token,
  change_program_brk,
  add_task,
};
use crate::timer::get_time_ms;

pub fn sys_getpid() -> isize {
  get_current_pid()
}

pub fn sys_fork() -> isize {
  let forking_task = get_current_task().fork();
  let child_pid = forking_task.pid.0;
  let trap_cx = forking_task.inner_exclusive_access().get_trap_cx();

  // set a0 register as 0 for return value for child proc
  trap_cx.regs[10] = 0;
  add_task(forking_task);

  child_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
  let token = get_current_token();
  let path = translate_str(token, path);
  if let Some(data) = get_app_data_by_name(path.as_str()) {
    get_current_task().exec(data);
    0
  } else {
    -1
  }
}

pub fn sys_waitpid(pid: usize) -> isize {
  unimplemented!()
}

pub fn sys_exit(xstate: i32) -> ! {
  info!("[kernel] Application {} exited with code {}", get_current_pid(), xstate);
  exit_current_and_run_next()
}

pub fn sys_get_taskinfo() -> isize {
  get_current_pid()
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
    return old_brk as isize;
  } else {
    -1
  }
}
