use riscv::register::sstatus::{self, SPP, Sstatus};

#[repr(C)]
pub struct TrapContext {
  pub regs: [usize; 32],
  pub sstatus: Sstatus,
  pub sepc: usize,
}

impl TrapContext {
  pub fn set_sp(&mut self, sp: usize) {
    self.regs[2] = sp;
  }

  pub fn app_init_context(entry: usize, user_sp: usize) -> Self {
    let mut sstatus = sstatus::read();
    sstatus.set_spp(SPP::User);
    let mut cx = Self {
      regs: [0; 32],
      sstatus,
      sepc: entry,
    };
    cx.set_sp(user_sp);
    cx
  }
}
