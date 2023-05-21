use core::arch::{asm, global_asm};
use log::debug;
use riscv::register::{stvec::TrapMode, scause::{
  Exception,
  Trap,
}, stval, stvec, scause, sie, sepc, sstatus};
use riscv::register::scause::Interrupt;
use crate::common::{intr_get, intr_off, intr_on};
use crate::config::*;
use crate::mm::VirtAddr;
use crate::syscall::syscall;
use crate::task::{exit, get_current_task, get_current_tcb_ref, get_current_token, get_current_trap_cx, yield_};
#[cfg(feature = "sbrk_lazy_alloc")]
use crate::task::lazy_alloc_page;
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

pub fn disable_timer_interrupt() {
  unsafe {
    sie::clear_stimer();
  }
}

#[no_mangle]
pub fn trap_handler() -> ! {
  // set trap entry to trap_from_kernel()
  set_kernel_trap_entry();
  let scause = scause::read();
  let stval = stval::read();
  let mut cx = get_current_trap_cx();
  match scause.cause() {
    Trap::Exception(Exception::UserEnvCall) => {
      // syscall
      cx.sepc += 4;
      intr_on();
      let result = syscall(cx.regs[17], [cx.regs[10], cx.regs[11], cx.regs[12]]) as usize;
      if cx.regs[17] == 221 {
        cx = get_current_trap_cx();
      }
      cx.regs[10] = result;
    }
    Trap::Exception(Exception::StoreFault)
    | Trap::Exception(Exception::StorePageFault)
    | Trap::Exception(Exception::LoadFault)
    | Trap::Exception(Exception::LoadPageFault) => {
      let tcb = get_current_tcb_ref();
      let tcb_inner = tcb.inner_borrow_ptr();
      let ok = if stval >= tcb_inner.heap_bottom && stval < tcb_inner.program_brk {
        // lazy allocation for sbrk()
        #[cfg(feature = "sbrk_lazy_alloc")] {
          lazy_alloc_page(stval.into())
        }
        #[cfg(not(feature = "sbrk_lazy_alloc"))] {
          false
        }
      } else {
        let to_match = tcb_inner.memory_set.translate(VirtAddr::from(stval).floor());
        match to_match {
          Some(pte) if pte.is_valid() && pte.is_readable() && pte.is_cow_page() => {
            // copy on write
            true
          }
          _ => {
            false
          }
        }
      };
      if !ok {
        debug!("[kernel] PageFault in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.", stval, cx.sepc);
        exit(-2);
      }
    }
    Trap::Exception(Exception::IllegalInstruction) => {
      debug!("[kernel] IllegalInstruction in application, kernel killed it.");
      exit(-3);
    }
    Trap::Interrupt(Interrupt::SupervisorTimer) => {
      set_next_trigger();
      yield_();
    }
    _ => {
      debug!("Unsupported trap {:?}, stval = {:#x}", scause.cause(), stval);
      exit(-1);
    }
  }
  trap_return()
}

#[no_mangle]
pub fn fork_ret() -> ! {
  let cur_task = get_current_task();
  cur_task.unlock();
  drop(cur_task);
  trap_return()
}

#[no_mangle]
pub fn trap_return() -> ! {
  intr_off();
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
  let sepc = sepc::read();
  let sstatus = sstatus::read();
  let scause = scause::read();

  if intr_get() != false {
    panic!("kerneltrap: interrupts enabled");
  }

  match scause.cause() {
    Trap::Exception(Exception::UserEnvCall) => {
      // syscall
      panic!("a syscall from kernel!");
    }
    Trap::Exception(Exception::StoreFault)
    | Trap::Exception(Exception::StorePageFault)
    | Trap::Exception(Exception::LoadFault)
    | Trap::Exception(Exception::LoadPageFault) => {
      panic!("a page fault from kernel!");
    }
    Trap::Exception(Exception::IllegalInstruction) => {
      panic!("a illegal instruction from kernel!");
    }
    Trap::Interrupt(Interrupt::SupervisorTimer) => {
      set_next_trigger();
      yield_();
    }
    _ => {}
  }

  sepc::write(sepc);
  unsafe {
    asm!(
    "csrw sstatus, {}", in(reg) sstatus.bits(),
    );
  }
  panic!("a trap from kernel!");
}
