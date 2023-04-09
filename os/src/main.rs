#![feature(panic_info_message)]
#![no_std]
#![no_main]

mod console;
mod lang_items;
mod sbi;
mod logging;
mod sync;
mod trap;
mod syscall;
mod stack_trace;
mod config;
mod loader;
mod task;
mod timer;

use core::arch::global_asm;
use log::{debug, error, info, LevelFilter, trace, warn};

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

extern "C" {
  fn stext();
  fn etext();
  fn srodata();
  fn erodata();
  fn sdata();
  fn edata();
  fn boot_stack_top();
  fn boot_stack_lower_bound();
  fn sbss();
  fn ebss();
}

pub fn clear_bss() {
  (sbss as usize..ebss as usize).for_each(|x| {
    unsafe { (x as *mut u8).write_volatile(0) }
  });
}

#[no_mangle]
pub fn rust_main() -> ! {
  clear_bss();
  logging::init(LevelFilter::Info.into());
  println!("hello world");
  trace!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize,
        etext as usize
    );
  debug!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
  info!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
  warn!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as usize, boot_stack_lower_bound as usize
    );
  error!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
  trap::init();
  loader::load_apps();
  trap::enable_timer_interrupt();
  timer::set_next_trigger();
  task::run_first_task();
  panic!("Unreachable in rust_main")
}
