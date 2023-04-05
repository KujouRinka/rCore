use core::arch::global_asm;
use log::warn;
use riscv::register::{
  stvec::TrapMode,
  scause::{Exception, Trap},
  stval,
  stvec,
  scause,
};
use crate::batch::run_next_app;
use crate::trap::context::TrapContext;
use crate::syscall::syscall;

pub mod context;

global_asm!(include_str!("trap.S"));

pub fn init() {
  extern "C" { fn __alltraps(); }
  unsafe {
    stvec::write(__alltraps as usize, TrapMode::Direct);
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
      run_next_app();
    }
    Trap::Exception(Exception::IllegalInstruction) => {
      warn!("[kernel] IllegalInstruction in application, kernel killed it.");
      run_next_app();
    }
    _ => {
      panic!("Unsupported trap {:?}, stval = {:#x}", scause.cause(), stval);
    }
  }
  cx
}
