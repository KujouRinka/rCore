use log::trace;

mod heap_allocator;
mod address;
mod page_table;
mod frame_allocator;
mod memory_set;

pub use address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum};
pub use frame_allocator::{frame_alloc, FrameTracker};
pub use memory_set::remap_test;
pub use memory_set::{MapPermission, MemorySet, KERNEL_SPACE};
pub use page_table::{translated_byte_buffer, translated_str, translated_copyout, PageTableEntry};

pub fn init() {
  heap_allocator::init_heap();
  trace!("heap_allocator inited");
  frame_allocator::init_frame_allocator();
  trace!("frame_allocator inited");
  KERNEL_SPACE.exclusive_access().activate();
  trace!("kernel_space inited");
}

#[allow(unused)]
pub fn test() {
  heap_allocator::heap_test();
  frame_allocator::frame_allocator_test();
  memory_set::remap_test();
}
