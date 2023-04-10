use core::arch::global_asm;
use log::warn;
use riscv::register::{stvec::TrapMode, scause::{Exception, Trap}, stval, stvec, scause, sie};
use riscv::register::scause::Interrupt;
use crate::trap::context::TrapContext;
use crate::syscall::{syscall, process::sys_exit};
use crate::task::{exit_current_and_run_next, suspend_current_and_run_next};
use crate::timer::set_next_trigger;

pub mod context;

global_asm!(include_str!("trap.S"));

pub fn init() {
  extern "C" { fn __alltraps(); }
  unsafe {
    stvec::write(__alltraps as usize, TrapMode::Direct);
  }
}

pub fn enable_timer_interrupt() {
  unsafe {
    sie::set_stimer();
  }
}

#[no_mangle]
pub fn trap_handler(cx: &mut TrapContext) -> &mut TrapContext {
  let scause = scause::read();
  let stval = stval::read();
  match scause.cause() {
    Trap::Exception(Exception::UserEnvCall) => {
      // syscall
      cx.sepc += 4;
      cx.regs[10] = syscall(cx.regs[17], [cx.regs[10], cx.regs[11], cx.regs[12]]) as usize;
    }
    Trap::Exception(Exception::StoreFault)
    | Trap::Exception(Exception::StorePageFault) => {
      warn!("[kernel] PageFault in application, kernel kill it.");
      sys_exit(1)
    }
    Trap::Exception(Exception::IllegalInstruction) => {
      warn!("[kernel] IllegalInstruction in application, kernel killed it.");
      sys_exit(1);
    }
    Trap::Interrupt(Interrupt::SupervisorTimer) => {
      set_next_trigger();
      suspend_current_and_run_next();
    }
    _ => {
      warn!("Unsupported trap {:?}, stval = {:#x}", scause.cause(), stval);
      sys_exit(1);
    }
  }
  cx
}
