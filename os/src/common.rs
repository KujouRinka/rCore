use core::arch::asm;

pub fn r_tp() -> usize {
  let tp: usize;
  unsafe {
    asm!("mv {}, tp", out(reg) tp);
  }
  tp
}
