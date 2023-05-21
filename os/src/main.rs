#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(stmt_expr_attributes)]
#![feature(negative_impls)]
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
mod common;
mod debug;

use core::arch::{asm, global_asm};
use core::sync::atomic::AtomicU32;
use core::sync::atomic::Ordering;
use log::{info, LevelFilter, trace};
use vars::*;
use crate::common::r_tp;
use crate::mm::KERNEL_SPACE;

global_asm!(include_str!("entry.asm"));
global_asm!(include_str!("link_app.S"));

pub fn clear_bss() {
  (sbss as usize..ebss as usize).for_each(|x| {
    unsafe { (x as *mut u8).write_volatile(0) }
  });
}

static STARTED: AtomicU32 = AtomicU32::new(0);

#[no_mangle]
pub fn rust_main(hartid: usize, _dtb: usize) -> ! {
  save_hartid_to_tp(hartid);
  if r_tp() == 0 {
    clear_bss();
    logging::init(LevelFilter::Off.into());
    info!("bss cleaned");
    mm::init();
    info!("mm inited");
    mm::test();
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
    STARTED.store(1, Ordering::Release);
  } else {
    while STARTED.load(Ordering::Acquire) == 0 {}
    trace!("hartid {} starting", r_tp());
    KERNEL_SPACE.lock().activate();
    trap::init();
    trap::enable_timer_interrupt();
    timer::set_next_trigger();
  }
  task::scheduler();
  panic!("Unreachable in rust_main")
}

fn save_hartid_to_tp(hartid: usize) {
  unsafe {
    asm!(
    "mv tp, {0}",
    in(reg) hartid,
    );
  }
}
