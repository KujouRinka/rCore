use core::arch::global_asm;
use log::warn;
use riscv::register::{
  stvec::TrapMode,
  scause::{
    Exception,
    Trap,
  },
  stval,
  stvec,
  scause,
  sie,
};
use riscv::register::scause::Interrupt;
use crate::config::*;
use crate::syscall::syscall;
use crate::task::{
  exit_current_and_run_next,
  get_current_token,
  get_current_trap_cx,
  suspend_current_and_run_next,
};
use crate::timer::set_next_trigger;

pub mod context;

global_asm!(include_str!("trap.S"));

extern "C" {
  fn __alltraps();
  fn __restore();
}

pub fn init() {
  set_user_trap_entry();
}

pub fn set_kernel_trap_entry() {
  unsafe {
    stvec::write(trap_from_kernel as usize, TrapMode::Direct);
  }
}

pub fn set_user_trap_entry() {
  unsafe {
    stvec::write(TRAMPOLINE, TrapMode::Direct);
  }
}

pub fn enable_timer_interrupt() {
  unsafe {
    sie::set_stimer();
  }
}

#[no_mangle]
pub fn trap_handler() -> ! {
  // set trap entry to trap_from_kernel()
  set_kernel_trap_entry();
  let scause = scause::read();
  let stval = stval::read();
  let cx = get_current_trap_cx();
  match scause.cause() {
    Trap::Exception(Exception::UserEnvCall) => {
      // syscall
      cx.sepc += 4;
      cx.regs[10] = syscall(cx.regs[17], [cx.regs[10], cx.regs[11], cx.regs[12]]) as usize;
    }
    Trap::Exception(Exception::StoreFault)
    | Trap::Exception(Exception::StorePageFault) => {
      warn!("[kernel] PageFault in application, kernel kill it.");
      exit_current_and_run_next();
    }
    Trap::Exception(Exception::IllegalInstruction) => {
      warn!("[kernel] IllegalInstruction in application, kernel killed it.");
      exit_current_and_run_next();
    }
    Trap::Interrupt(Interrupt::SupervisorTimer) => {
      set_next_trigger();
      suspend_current_and_run_next();
    }
    _ => {
      warn!("Unsupported trap {:?}, stval = {:#x}", scause.cause(), stval);
      exit_current_and_run_next();
    }
  }
  trap_return()
}

#[no_mangle]
pub fn trap_return() -> ! {
  set_user_trap_entry();
  let trap_cx_ptr_for_va = TRAP_CONTEXT;
  let user_satp = get_current_token();
  let restore_va = TRAMPOLINE + (__restore as usize - __alltraps as usize);
  let restore_fn =
    unsafe { core::mem::transmute::<_, extern "C" fn(usize, usize) -> !>(restore_va) };
  restore_fn(trap_cx_ptr_for_va, user_satp)
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
  panic!("a trap from kernel!");
}
