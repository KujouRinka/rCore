use crate::task::context::TaskContext;

#[derive(Copy, Clone, PartialEq, Eq)]
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
}
