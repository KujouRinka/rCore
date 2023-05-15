#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(stmt_expr_attributes)]
#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
extern crate bitflags;

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
mod mm;
mod vars;

use core::arch::global_asm;
use log::{info, LevelFilter};
use vars::*;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

pub fn clear_bss() {
  (sbss as usize..ebss as usize).for_each(|x| {
    unsafe { (x as *mut u8).write_volatile(0) }
  });
}

#[no_mangle]
pub fn rust_main() -> ! {
  clear_bss();
  logging::init(LevelFilter::Trace.into());
  info!("bss cleaned");
  mm::init();
  info!("mm inited");
  mm::test();
  println!("hello world");
  info!(
        "[kernel] .text [{:#x}, {:#x})",
        stext as usize,
        etext as usize
    );
  info!(
        "[kernel] .rodata [{:#x}, {:#x})",
        srodata as usize, erodata as usize
    );
  info!(
        "[kernel] .data [{:#x}, {:#x})",
        sdata as usize, edata as usize
    );
  info!(
        "[kernel] boot_stack top=bottom={:#x}, lower_bound={:#x}",
        boot_stack_top as usize, boot_stack_lower_bound as usize
    );
  info!("[kernel] .bss [{:#x}, {:#x})", sbss as usize, ebss as usize);
  trap::init();
  info!("trap inited");
  trap::enable_timer_interrupt();
  info!("timer interrupt opened");
  timer::set_next_trigger();
  task::init();
  info!("being able to run initproc");
  task::scheduler();
  panic!("Unreachable in rust_main")
}
