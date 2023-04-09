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

  pub fn goto_restore(kstack_ptr: usize) -> Self {
    extern "C" {
      fn __restore();
    }
    Self {
      ra: __restore as usize,
      sp: kstack_ptr,
      regs: [0; 12],
    }
  }
}
