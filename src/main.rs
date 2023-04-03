#![feature(panic_info_message)]
#![no_std]
#![no_main]

mod console;
mod lang_items;
mod sbi;

use core::arch::global_asm;
use crate::sbi::shutdown;

global_asm!(include_str!("entry.asm"));

pub fn clear_bss() {
  extern "C" {
    fn sbss();
    fn ebss();
  }
  (sbss as usize..ebss as usize).for_each(|x| {
    unsafe { (x as *mut u8).write_volatile(0) }
  });
}

#[no_mangle]
pub fn rust_main() -> ! {
  clear_bss();
  println!("hello world");
  shutdown()
}
