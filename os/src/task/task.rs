use crate::syscall::{MAX_SYSCALL_NUM, SyscallInfo};
use crate::task::context::TaskContext;

#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C)]
pub enum TaskStatus {
  UnInit,
  Ready,
  Running,
  Exited,
}

#[derive(Copy, Clone)]
pub struct TaskControlBlock {
  pub task_status: TaskStatus,
  pub task_cx: TaskContext,
  pub call: [SyscallInfo; MAX_SYSCALL_NUM],
  pub start_time: usize,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskInfo {
  pub id: usize,
  pub status: TaskStatus,
  pub call: [SyscallInfo; MAX_SYSCALL_NUM],
  pub time: usize,
}

