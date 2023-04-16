use crate::trap::trap_return;

#[derive(Copy, Clone)]
#[repr(C)]
pub struct TaskContext {
  ra: usize,
  sp: usize,
  regs: [usize; 12],
}

impl TaskContext {
  pub fn zero_init() -> Self {
    Self {
      ra: 0,
      sp: 0,
      regs: [0; 12],
    }
  }

  pub fn goto_trap_return(kstack_ptr: usize) -> Self {
    Self {
      ra: trap_return as usize,
      sp: kstack_ptr,
      regs: [0; 12],
    }
  }
}
