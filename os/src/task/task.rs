use core::ptr;
use crate::config::*;
use crate::mm::{KERNEL_SPACE, MapPermission, MemorySet, PhysPageNum, VirtAddr, VirtPageNum};
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
  pub heap_bottom: usize,
  pub program_brk: usize,   // also heap_top
}

impl TaskControlBlock {
  pub fn new(elf_data: &[u8], app_id: usize) -> Self {
    let (memory_set, user_stack_top, heap_bottom, entry_point) = MemorySet::from_elf(elf_data);
    let trap_cx_ppn = memory_set
      .translate(VirtAddr::from(TRAP_CONTEXT).into())
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
      heap_bottom,
      program_brk: heap_bottom,
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

  pub fn change_brk(&mut self, size: i32) -> Option<usize> {
    let old_brk = self.program_brk;
    let new_brk = self.program_brk as isize + size as isize;
    if new_brk < self.heap_bottom as isize {
      return None;
    }
    // let result = if size < 0 {
    //   self.memory_set.shrink_to(VirtAddr::from(self.heap_bottom), VirtAddr::from(new_brk as usize))
    // } else {
    //   self.memory_set.append_to(VirtAddr::from(self.heap_bottom), VirtAddr::from(new_brk as usize))
    // };
    // TODO: free pages possible when shrink.
    let result = true;
    if result {
      self.program_brk = new_brk as usize;
      Some(old_brk)
    } else {
      None
    }
  }

  pub fn lazy_alloc_page(&mut self, vpn: VirtPageNum) -> bool {
    unsafe {
      self.memory_set.insert_framed_area(
        vpn.into(),
        (VirtAddr::from(vpn).0 + 1).into(),
        MapPermission::R | MapPermission::W | MapPermission::U,
      );
    }
    true
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
