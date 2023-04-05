#![no_std]
#![no_main]

use core::arch::asm;

#[macro_use]
extern crate user_lib;

#[no_mangle]
fn main() -> i32 {
  let mut cur_sp: usize;
  // load riscv64 stack pointer
  unsafe {
    asm!("mv {}, sp", out(reg) cur_sp);
  }
  println!("{:x}", cur_sp);
  0
}