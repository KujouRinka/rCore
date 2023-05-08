use alloc::sync::{Weak, Arc};
use alloc::vec::Vec;
use core::cell::{Ref, RefMut};
use core::ptr;
use cfg_if::cfg_if;
use crate::config::*;
use crate::mm::{KERNEL_SPACE, MapPermission, MemorySet, PhysPageNum, VirtAddr};
use crate::sync::UPSafeCell;
use crate::task::context::TaskContext;
use crate::trap::context::TrapContext;
use crate::trap::trap_handler;
use crate::task::pid::{kernel_stack_position, KernelStack, PidHandle};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskStatus {
  Ready,
  Running,
  Zombie,
  Exited,
}

pub struct TaskControlBlock {
  // immutable
  pub pid: PidHandle,
  pub kernel_stack: KernelStack,
  // mutable
  inner: UPSafeCell<TaskControlBlockInner>,
}

impl TaskControlBlock {
  /// Only used for creating initproc
  pub fn new(elf_data: &[u8]) -> Self {
    unimplemented!()
  }

  pub fn exec(&self, elf_data: &[u8]) {
    unimplemented!()
  }

  pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
    unimplemented!()
  }

  pub fn inner_borrow(&self) -> Ref<'_, TaskControlBlockInner> {
    self.inner.borrow()
  }

  pub fn inner_exclusive_access(&self) -> RefMut<'_, TaskControlBlockInner> {
    self.inner.exclusive_access()
  }

  pub fn get_pid(&self) -> usize {
    return self.pid.0;
  }

  pub fn change_brk(&mut self, size: i32) -> Option<usize> {
    self.inner_exclusive_access().change_brk(size)
  }

  #[cfg(feature = "sbrk_lazy_alloc")]
  pub fn lazy_alloc_page(&mut self, addr: VirtAddr) -> bool {
    self.inner_exclusive_access().lazy_alloc_page(addr)
  }
}

pub struct TaskControlBlockInner {
  // Used for __switch
  pub task_status: TaskStatus,
  pub task_cx: TaskContext,
  // Used for mm
  pub memory_set: MemorySet,
  pub trap_cx_ppn: PhysPageNum,
  pub heap_bottom: usize,
  pub program_brk: usize,
  // also heap_top
  // Used for process
  pub parent: Option<Weak<TaskControlBlock>>,
  pub children: Vec<Arc<TaskControlBlock>>,
  pub xcode: i32,
}

impl TaskControlBlockInner {
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
      parent: None,
      children: Vec::new(),
      xcode: -1,
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
    let ok;
    cfg_if! {
      if #[cfg(feature = "sbrk_lazy_alloc")] {
        // TODO: free pages possible when shrink.
        ok = true;
        if size < 0 {
          self.memory_set
            .remove_framed_area(
              VirtAddr::from(new_brk as usize),
              VirtAddr::from(self.program_brk),
            );
        }
      } else {
        ok = if size < 0 {
          self.memory_set.shrink_to(VirtAddr::from(self.heap_bottom), VirtAddr::from(new_brk as usize))
        } else {
          self.memory_set.append_to(VirtAddr::from(self.heap_bottom), VirtAddr::from(new_brk as usize))
        }
      }
    }
    if ok {
      self.program_brk = new_brk as usize;
      Some(old_brk)
    } else {
      None
    }
  }

  #[cfg(feature = "sbrk_lazy_alloc")]
  pub fn lazy_alloc_page(&mut self, addr: VirtAddr) -> bool {
    unsafe {
      self.memory_set.insert_framed_area(
        addr,
        (addr.0 + 1).into(),
        MapPermission::R | MapPermission::W | MapPermission::U,
      );
    }
    true
  }
}

impl TaskControlBlockInner {
  pub fn get_user_token(&self) -> usize {
    self.memory_set.token()
  }

  pub fn get_trap_cx(&self) -> &'static mut TrapContext {
    self.trap_cx_ppn.get_mut()
  }

  fn get_status(&self) -> TaskStatus {
    return self.task_status;
  }

  pub fn is_zombie(&self) -> bool {
    self.get_status() == TaskStatus::Zombie
  }
}
