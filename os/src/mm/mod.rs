use log::trace;
use crate::mm::memory_set::KERNEL_SPACE;

mod heap_allocator;
mod address;
mod page_table;
mod frame_allocator;
mod memory_set;

pub fn init() {
  trace!("initing mm");
  heap_allocator::init_heap();
  trace!("heap_allocator inited");
  frame_allocator::init_frame_allocator();
  trace!("frame_allocator inited");
  KERNEL_SPACE.exclusive_access().activate();
  trace!("kernel_space inited");
  trace!("mm inited");
}

#[allow(unused)]
pub fn test() {
  heap_allocator::heap_test();
  frame_allocator::frame_allocator_test();
  memory_set::remap_test();
}
