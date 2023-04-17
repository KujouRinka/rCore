use alloc::vec::Vec;
use core::fmt::{self, Debug, Formatter};
use lazy_static::lazy_static;
use log::trace;
use crate::config::MEMORY_END;
use crate::mm::address::{PhysAddr, PhysPageNum};
use crate::sync::UPSafeCell;
use crate::vars::*;

trait FrameAllocator {
  fn new() -> Self;
  fn alloc(&mut self) -> Option<PhysPageNum>;
  fn dealloc(&mut self, ppn: PhysPageNum);
}

pub struct StackFrameAllocator {
  current: usize,
  end: usize,
  recycled: Vec<usize>,
}

impl StackFrameAllocator {
  pub fn init(&mut self, l: PhysPageNum, r: PhysPageNum) {
    self.current = l.0;
    self.end = r.0;
  }
}

impl FrameAllocator for StackFrameAllocator {
  fn new() -> Self {
    Self {
      current: 0,
      end: 0,
      recycled: Vec::new(),
    }
  }

  fn alloc(&mut self) -> Option<PhysPageNum> {
    if let Some(ppn) = self.recycled.pop() {
      Some(ppn.into())
    } else if self.current == self.end {
      None
    } else {
      self.current += 1;
      Some((self.current - 1).into())
    }
  }

  fn dealloc(&mut self, ppn: PhysPageNum) {
    let ppn = ppn.0;
    if ppn >= self.current || self.recycled
      .iter()
      .find(|&v| *v == ppn)
      .is_some() {
      panic!("Frame ppn={:#x} has not been allocated!", ppn);
    }
    self.recycled.push(ppn);
  }
}

type FrameAllocatorImpl = StackFrameAllocator;
lazy_static! {
  pub static ref FRAME_ALLOCATOR: UPSafeCell<FrameAllocatorImpl> = unsafe {
    UPSafeCell::new(FrameAllocatorImpl::new())
  };
}

pub fn init_frame_allocator() {
  FRAME_ALLOCATOR.exclusive_access()
    .init(
      PhysAddr::from(ekernel as usize).ceil(),
      PhysAddr::from(MEMORY_END).floor(),
    );
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameTracker {
  pub ppn: PhysPageNum,
}

pub struct FrameTrackerMarker {
  pub ppn: PhysPageNum,
}

impl FrameTrackerMarker {
  pub fn new(ppn: PhysPageNum) -> Self {
    Self { ppn }
  }

  /// Generate a key for Map/Set to delete without drop.
  pub fn frame_tracker_ref(&self) -> &'static FrameTracker {
    unsafe { core::mem::transmute(self) }
  }
}

impl FrameTracker {
  pub fn new(ppn: PhysPageNum) -> Self {
    let bytes_array = ppn.get_bytes_array();
    bytes_array.fill(0);
    Self { ppn }
  }
}

impl Debug for FrameTracker {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    f.write_fmt(format_args!("FrameTracker:PPN={:#x}", self.ppn.0))
  }
}

impl Drop for FrameTracker {
  fn drop(&mut self) {
    frame_dealloc(self.ppn);
  }
}

pub fn frame_alloc() -> Option<FrameTracker> {
  FRAME_ALLOCATOR
    .exclusive_access()
    .alloc()
    .map(FrameTracker::new)
}

pub fn frame_dealloc(ppn: PhysPageNum) {
  FRAME_ALLOCATOR
    .exclusive_access()
    .dealloc(ppn);
}

#[allow(unused)]
pub fn frame_allocator_test() {
  let mut v: Vec<FrameTracker> = Vec::new();
  for i in 0..5 {
    let frame = frame_alloc().unwrap();
    trace!("{:?}", frame);
    v.push(frame);
  }
  v.clear();
  for i in 0..5 {
    let frame = frame_alloc().unwrap();
    trace!("{:?}", frame);
    v.push(frame);
  }
  drop(v);
  trace!("frame_allocator_test passed!");
}
