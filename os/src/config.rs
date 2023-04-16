//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 1 << 13;
pub const KERNEL_STACK_SIZE: usize = 1 << 13;
pub const CLOCK_FREQ: usize = 12500000;

// mm
pub const KERNEL_HEAP_SIZE: usize = 0x30_0000;
pub const PAGE_SIZE_BITS: usize = 0xc;
pub const PAGE_SIZE: usize = 1 << PAGE_SIZE_BITS;   // 4k
pub const PTE_FLAGS_BITS: usize = 0xa;

pub const TRAMPOLINE: usize = usize::MAX - PAGE_SIZE + 1;
pub const TRAP_CONTEXT: usize = TRAMPOLINE - PAGE_SIZE;

// pub const MEMORY_END: usize = 0x88000000;  // 128M
pub const MEMORY_END: usize = 0x80800000;     // 8M

/// # Layout
/// ```
/// +-------------------+
/// |    Trampoline     |
/// |-------------------|
/// |    Guard Page     |
/// |-------------------|
/// |  Kernel Stack 0   |
/// |-------------------|
/// |    Guard Page     |
/// |-------------------|
/// |  Kernel Stack 1   |
/// |-------------------|
/// |        ...        |
/// |                   |
/// ```
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
  let top = TRAMPOLINE - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
  let bottom = top - KERNEL_STACK_SIZE;
  (bottom, top)
}
