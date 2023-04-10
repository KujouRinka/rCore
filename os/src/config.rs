//! Constants used in rCore

pub const USER_STACK_SIZE: usize = 1 << 13;
pub const KERNEL_STACK_SIZE: usize = 1 << 13;
pub const MAX_APP_NUM: usize = 7;
pub const APP_BASE_ADDRESS: usize = 0x80400000;
pub const APP_SIZE_LIMIT: usize = 0x20000;
pub const CLOCK_FREQ: usize = 12500000;
