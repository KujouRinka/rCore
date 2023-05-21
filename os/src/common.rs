use core::arch::asm;
use crate::task::current_cpu;
use crate::trap::{disable_timer_interrupt, enable_timer_interrupt};

pub fn r_tp() -> usize {
  let tp: usize;
  unsafe {
    asm!("mv {}, tp", out(reg) tp);
  }
  tp
}

pub fn r_sstatus() -> usize
{
  let sstatus: usize;
  unsafe {
    asm!("csrr {}, sstatus", out(reg) sstatus);
  }
  sstatus
}

pub fn cpuid() -> usize {
  r_tp()
}

pub fn intr_get() -> bool {
  let sstatus = r_sstatus();
  sstatus & (1 << 1) != 0
}

pub fn intr_off() {
  disable_timer_interrupt();
}

pub fn intr_on() {
  enable_timer_interrupt();
}

pub fn push_off() {
  let old = intr_get();
  intr_off();
  if current_cpu().noff == 0 {
    current_cpu().intena = old;
  }
  current_cpu().noff += 1;
}

pub fn pop_off() {
  let mycpu = current_cpu();
  if intr_get() {
    panic!("pop_off - interruptible");
  }
  if mycpu.noff < 1 {
    panic!("pop_off");
  }
  mycpu.noff -= 1;
  if mycpu.noff == 0 && mycpu.intena {
    intr_on();
  }
}
