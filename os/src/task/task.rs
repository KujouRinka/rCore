use alloc::sync::{Weak, Arc};
use alloc::vec::Vec;
use core::cell::{Ref, RefMut};
use core::ptr;
use cfg_if::cfg_if;
use crate::config::*;
use crate::mm::{KERNEL_SPACE, MapPermission, MemorySet, PhysPageNum, VirtAddr};
use crate::sync::UPSafeCell;
use crate::task::{
  context::TaskContext,
  pid::{kernel_stack_position, KernelStack, pid_alloc, PidHandle},
};
use crate::trap::{context::TrapContext, trap_handler};

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum TaskStatus {
  Ready,
  Running,
  Zombie,
  #[allow(unused)]
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
  pub fn new_for_initproc(elf_data: &[u8]) -> Self {
    let pid = pid_alloc();
    let kernel_stack = KernelStack::new(&pid);
    let inner = unsafe { UPSafeCell::new(TaskControlBlockInner::new(elf_data, pid.0)) };
    Self {
      pid,
      kernel_stack,
      inner,
    }
  }

  pub fn exec(&self, elf_data: &[u8]) {
    let (memory_set, user_stack_top, _, entry_point) = MemorySet::from_elf(elf_data);

    let trap_cx_ppn = memory_set.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();

    let mut inner = self.inner.borrow_mut();
    inner.trap_cx_ppn = trap_cx_ppn;
    inner.memory_set = memory_set;

    let trap_cx = inner.get_trap_cx();
    *trap_cx = TrapContext::app_init_context(
      entry_point,
      user_stack_top,
      KERNEL_SPACE.borrow_mut().token(),
      self.kernel_stack.get_top(),
      trap_handler as usize,
    );
  }

  pub fn fork(self: &Arc<TaskControlBlock>) -> Arc<TaskControlBlock> {
    let pid = pid_alloc();
    let kernel_stack = KernelStack::new(&pid);

    let mut parent_inner = self.inner_borrow_mut();
    let memory_set = MemorySet::from_another(&parent_inner.memory_set);
    let trap_cx_ppn = memory_set.translate(VirtAddr::from(TRAP_CONTEXT).into()).unwrap().ppn();
    let kernel_stack_top = kernel_stack.get_top();

    let tcb_inner = TaskControlBlockInner {
      task_status: TaskStatus::Ready,
      task_cx: TaskContext::goto_trap_return(kernel_stack_top),
      memory_set,
      trap_cx_ppn,
      heap_bottom: parent_inner.heap_bottom,
      program_brk: parent_inner.program_brk,
      parent: Some(Arc::downgrade(self)),
      children: Vec::new(),
      xcode: 0,
    };
    let new_tcb = TaskControlBlock {
      pid,
      kernel_stack,
      inner: unsafe { UPSafeCell::new(tcb_inner) },
    };
    new_tcb.inner_borrow_mut().get_trap_cx().kernel_sp = kernel_stack_top;
    let ret = Arc::new(new_tcb);
    parent_inner.children.push(Arc::clone(&ret));
    ret
  }

  pub fn inner_borrow(&self) -> Ref<'_, TaskControlBlockInner> {
    self.inner.borrow()
  }

  pub fn inner_borrow_mut(&self) -> RefMut<'_, TaskControlBlockInner> {
    self.inner.borrow_mut()
  }

  pub fn get_pid(&self) -> usize {
    return self.pid.0;
  }

  pub fn change_brk(&self, size: i32) -> Option<usize> {
    self.inner_borrow_mut().change_brk(size)
  }

  #[cfg(feature = "sbrk_lazy_alloc")]
  pub fn lazy_alloc_page(&self, addr: VirtAddr) -> bool {
    self.inner_borrow_mut().lazy_alloc_page(addr)
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
  pub fn new(elf_data: &[u8], pid: usize) -> Self {
    let (memory_set, user_stack_top, heap_bottom, entry_point) = MemorySet::from_elf(elf_data);
    let trap_cx_ppn = memory_set
      .translate(VirtAddr::from(TRAP_CONTEXT).into())
      .unwrap()
      .ppn();
    let (_, kernel_top) = kernel_stack_position(pid);
    let tcb = Self {
      task_status: TaskStatus::Ready,
      task_cx: TaskContext::goto_trap_return(kernel_top),
      memory_set,
      trap_cx_ppn,
      heap_bottom,
      program_brk: heap_bottom,
      parent: None,
      children: Vec::new(),
      xcode: 0,
    };
    let trap_cx = tcb.get_trap_cx();
    let to_write_cx = TrapContext::app_init_context(
      entry_point,
      user_stack_top,
      KERNEL_SPACE.borrow_mut().token(),
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
