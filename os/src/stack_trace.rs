use core::arch::asm;
use crate::println;

pub unsafe fn print_stack_trace() {
  let mut cur_fp: usize;
  // load riscv64 stack pointer
  asm!("mv {}, fp", out(reg) cur_fp);
  let mut cnt = 0;
  println!("== Begin stack trace ==");
  while cur_fp != 0 {
    unsafe {
      let ra = *((cur_fp - 8) as *const usize);
      cur_fp = *((cur_fp - 16) as *const usize);
      println!("{}: ra=0x{:016x}, fp=0x{:016x}", cnt, ra, cur_fp);
    }
    cnt += 1;
  }
  println!("== End stack trace ==");
}
