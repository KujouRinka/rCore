use alloc::collections::VecDeque;
use alloc::sync::Arc;
use lazy_static::lazy_static;
use crate::sync::SpinMutex;
use crate::task::task::TaskControlBlock;

lazy_static! {
  pub static ref TASK_MANAGER: SpinMutex<TaskManager> =
    SpinMutex::new(TaskManager::new());
}

pub struct TaskManager {
  ready_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl TaskManager {
  fn new() -> Self {
    Self { ready_queue: VecDeque::new() }
  }

  fn add(&mut self, task: Arc<TaskControlBlock>) {
    self.ready_queue.push_back(task);
  }

  fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
    self.ready_queue.pop_front()
  }
}

pub fn add_task(task: Arc<TaskControlBlock>) {
  TASK_MANAGER.lock().add(task);
}

pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
  TASK_MANAGER.lock().fetch()
}
