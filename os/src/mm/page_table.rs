use alloc::collections::BTreeSet;
use bitflags::*;
use crate::config::PTE_FLAGS_BITS;
use crate::mm::address::{PhysPageNum, VirtPageNum};
use crate::mm::frame_allocator::{frame_alloc, FrameTracker, FrameTrackerMarker};

bitflags! {
  pub struct PTEFlags: u8 {
    const V = 1 << 0;
    const R = 1 << 1;
    const W = 1 << 2;
    const X = 1 << 3;
    const U = 1 << 4;
    const G = 1 << 5;
    const A = 1 << 6;
    const D = 1 << 7;
  }
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct PageTableEntry {
  pub bits: usize,
}

impl PageTableEntry {
  pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
    Self {
      bits: ppn.0 << PTE_FLAGS_BITS | flags.bits as usize,
    }
  }

  pub fn empty() -> Self {
    Self { bits: 0 }
  }

  pub fn ppn(&self) -> PhysPageNum {
    (self.bits >> PTE_FLAGS_BITS & (1usize << 44) - 1).into()
  }

  pub fn flags(&self) -> PTEFlags {
    PTEFlags::from_bits(self.bits as u8).unwrap()
  }

  pub fn is_valid(&self) -> bool {
    (self.flags() & PTEFlags::V) != PTEFlags::empty()
  }

  pub fn is_readable(&self) -> bool {
    (self.flags() & PTEFlags::R) != PTEFlags::empty()
  }

  pub fn is_writable(&self) -> bool {
    (self.flags() & PTEFlags::W) != PTEFlags::empty()
  }

  pub fn is_executable(&self) -> bool {
    (self.flags() & PTEFlags::X) != PTEFlags::empty()
  }
}

pub struct PageTable {
  root_ppn: PhysPageNum,
  frames_holder: BTreeSet<FrameTracker>,
}

impl PageTable {
  pub fn new() -> Self {
    let frame = frame_alloc().unwrap();
    Self {
      root_ppn: frame.ppn,
      frames_holder: {
        let mut b = BTreeSet::new();
        b.insert(frame);
        b
      },
    }
  }

  pub fn from_token(satp: usize) -> Self {
    Self {
      root_ppn: satp.into(),
      frames_holder: BTreeSet::new(),
    }
  }

  pub fn token(&self) -> usize {
    8usize << 60 | self.root_ppn.0
  }

  pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
    self.find_pte(vpn).map(|pte| *pte)
  }

  pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags, mut frame: Option<FrameTracker>) {
    let pte = self.find_pte_create(vpn).unwrap();
    assert!(!pte.is_valid(), "vpn {:?} is mapped but should not", vpn);

    // update pte permission
    *pte = PageTableEntry::new(ppn, flags | PTEFlags::V);
    // hold this page
    if let Some(ft) = frame.take() {
      assert_eq!(ppn, ft.ppn, "map: ppn and frame.ppn should equal");
      self.frames_holder.insert(ft);
    }
  }

  pub fn unmap(&mut self, vpn: VirtPageNum, dealloc: bool) {
    let pte = self.find_pte(vpn).unwrap();
    assert!(pte.is_valid(), "vpn {:?} should mapped but not", vpn);
    *pte = PageTableEntry::empty();
    if dealloc {
      // TODO: release record in self.frames
      let key_to_remove = FrameTrackerMarker::new(pte.ppn()).frame_tracker_ref();
      self.frames_holder.remove(&key_to_remove);
    }
  }
}

impl PageTable {
  /// Create a new VA to PA, create a map if not exist but not alloc actual page.
  fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
    let index = vpn.indexes();
    let mut ppn = self.root_ppn.clone();
    let mut ret = None;
    for (i, idx) in index.into_iter().enumerate() {
      let next_pte = &mut ppn.get_pte_array()[idx];
      if !next_pte.is_valid() && i != 2 {
        let new_frame = frame_alloc().unwrap();
        *next_pte = PageTableEntry::new(new_frame.ppn, PTEFlags::V);
        self.frames_holder.insert(new_frame);
      }
      ppn = next_pte.ppn();
      ret = Some(next_pte);
    }
    ret
  }

  fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
    let index = vpn.indexes();
    let mut ppn = self.root_ppn.clone();
    let mut ret = None;
    for (i, idx) in index.into_iter().enumerate() {
      let next_pte = &mut ppn.get_pte_array()[idx];
      if !next_pte.is_valid() && i != 2 {
        return None;
      }
      ppn = next_pte.ppn();
      ret = Some(next_pte);
    }
    ret
  }

  pub fn find_ppn(&self, vpn: VirtPageNum) -> Option<PhysPageNum> {
    self.find_pte(vpn).map(|pte| pte.ppn())
  }
}
