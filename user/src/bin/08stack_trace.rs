#![no_std]
#![no_main]

use core::arch::asm;

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
  // print_stack();
  recursive(2);
  0
}

fn recursive(i: i32) {
  if i == 0 {
    unsafe {
      print_stack_trace();
    }
    return;
  }
  // run this without optimization
  recursive(i - 1);
  recursive(i - 1);
}

unsafe fn print_stack_trace() {
  let mut cur_fp: usize;
  // load riscv64 stack pointer
  asm!("mv {}, fp", out(reg) cur_fp);
  let mut cnt = 0;
  while cur_fp != 0 {
    unsafe {
      let ra = *((cur_fp - 8) as *const usize);
      cur_fp = *((cur_fp - 16) as *const usize);
      println!("{}: 0x{:016x}, fp=0x{:016x}", cnt, ra, cur_fp);
    }
    cnt += 1;
  }
}
