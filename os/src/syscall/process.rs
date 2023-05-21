use alloc::sync::Arc;
use crate::loader::get_app_data_by_name;
use crate::mm::{translated_str, translated_copyout};
use crate::task::{
  get_current_task,
  yield_,
  exit,
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
  let forking_task_inner = forking_task.inner_borrow_ptr();
  let trap_cx = forking_task_inner.get_trap_cx();

  // set a0 register as 0 for return value for child proc
  trap_cx.regs[10] = 0;
  add_task(forking_task);

  child_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
  let token = get_current_token();
  let path = translated_str(token, path);
  if let Some(data) = get_app_data_by_name(path.as_str()) {
    get_current_task().exec(data);
    0
  } else {
    -1
  }
}

pub fn sys_waitpid(pid: isize, xcode_ptr: *mut i32) -> isize {
  let task = get_current_task();
  task.lock();
  let task_inner = task.inner_borrow_ptr_mut();
  if !task_inner.children.iter()
    .any(|p| {
      let ret = pid == -1 || pid as usize == p.get_pid();
      ret
    }) {
    task.unlock();
    return -1;
  }
  let pair = task_inner.children.iter()
    .enumerate()
    .find(|(_, p)| {
      p.lock();
      let ret = p.inner_borrow_ptr().is_zombie() && (pid == -1 || pid as usize == p.get_pid());
      if !ret {
        p.unlock();
      }
      ret
    });
  if let Some((idx, _)) = pair {
    let child = task_inner.children.remove(idx);
    assert_eq!(Arc::strong_count(&child), 1);

    let found_pid = child.get_pid();
    let child_inner = child.inner_borrow_ptr();
    let xcode = child_inner.xcode;

    translated_copyout(task_inner.get_user_token(), xcode_ptr, xcode);

    child.unlock();
    drop(child);

    task.unlock();
    found_pid as isize
  } else {
    task.unlock();
    -2
  }
}

pub fn sys_exit(xcode: i32) -> ! {
  exit(xcode)
}

pub fn sys_get_taskinfo() -> isize {
  get_current_pid()
}

pub fn sys_yield() -> isize {
  yield_();
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
