mod heap_allocator;

pub use heap_allocator::heap_test;

pub fn init() {
  heap_allocator::init_heap();
}
