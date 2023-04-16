use core::ptr;
use crate::config::*;
use crate::mm::{KERNEL_SPACE, MapPermission, MemorySet, PhysPageNum};
use crate::task::context::TaskContext;
use crate::trap::context::TrapContext;
use crate::trap::trap_handler;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskStatus {
  Ready,
  Running,
  Exited,
}

pub struct TaskControlBlock {
  // Used for __switch
  pub task_status: TaskStatus,
  pub task_cx: TaskContext,
  // Used for mm
  pub memory_set: MemorySet,
  pub trap_cx_ppn: PhysPageNum,
  pub base_size: usize,
}

impl TaskControlBlock {
  pub fn new(elf_data: &[u8], app_id: usize) -> Self {
    let (memory_set, user_stack_top, entry_point) = MemorySet::from_elf(elf_data);
    let trap_cx_ppn = memory_set
      .translate(TRAP_CONTEXT.into())
      .unwrap()
      .ppn();
    let (kernel_bottom, kernel_top) = kernel_stack_position(app_id);
    unsafe {
      KERNEL_SPACE.exclusive_access()
        .insert_framed_area(
          kernel_bottom.into(),
          kernel_top.into(),
          MapPermission::R | MapPermission::W,
        );
    }
    let tcb = Self {
      task_status: TaskStatus::Ready,
      task_cx: TaskContext::goto_trap_return(kernel_top),
      memory_set,
      trap_cx_ppn,
      base_size: user_stack_top,
    };
    let trap_cx = tcb.get_trap_cx();
    let to_write_cx = TrapContext::app_init_context(
      entry_point,
      user_stack_top,
      KERNEL_SPACE.exclusive_access().token(),
      kernel_top,
      trap_handler as usize,
    );
    unsafe {
      ptr::write_volatile(trap_cx, to_write_cx);
    }
    tcb
  }
}

impl TaskControlBlock {
  pub fn get_user_token(&self) -> usize {
    self.memory_set.token()
  }

  pub fn get_trap_cx(&self) -> &'static mut TrapContext {
    self.trap_cx_ppn.get_mut()
  }
}
