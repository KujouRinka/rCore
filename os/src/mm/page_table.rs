use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::vec::Vec;
use bitflags::*;
use crate::config::{PAGE_SIZE, PTE_FLAGS_BITS};
use crate::mm::address::{PhysPageNum, VirtPageNum};
use crate::mm::frame_allocator::{frame_alloc, FrameTracker, FrameTrackerMarker};
use crate::mm::{PhysAddr, VirtAddr};

bitflags! {
  pub struct PTEFlags: u16 {
    const V = 1 << 0;
    const R = 1 << 1;
    const W = 1 << 2;
    const X = 1 << 3;
    const U = 1 << 4;
    const G = 1 << 5;
    const A = 1 << 6;
    const D = 1 << 7;
    const C = 1 << 8;
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
    PTEFlags::from_bits((self.bits & (1 << 10) - 1) as u16).unwrap()
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

  pub fn is_cow_page(&self) -> bool {
    (self.flags() & PTEFlags::C) != PTEFlags::empty()
  }
}

pub struct MapArgs {
  vpn: VirtPageNum,
  ppn: PhysPageNum,
  flags: PTEFlags,
  frame: Option<FrameTracker>,
}

pub struct UnmapArgs {
  vpn: VirtPageNum,
  dealloc: bool,
  panic: bool,
}

impl MapArgs {
  pub fn builder(vpn: VirtPageNum, ppn: PhysPageNum) -> Self {
    Self {
      vpn,
      ppn,
      flags: PTEFlags::empty(),
      frame: None,
    }
  }

  #[allow(unused)]
  pub fn with_vpn(mut self, vpn: VirtPageNum) -> Self {
    self.vpn = vpn;
    self
  }

  #[allow(unused)]
  pub fn with_ppn(mut self, ppn: PhysPageNum) -> Self {
    self.ppn = ppn;
    self
  }

  pub fn with_flags(mut self, flags: PTEFlags) -> Self {
    self.flags = flags;
    self
  }

  pub fn with_frame(mut self, frame: Option<FrameTracker>) -> Self {
    self.frame = frame;
    self
  }
}

impl UnmapArgs {
  pub fn builder(vpn: VirtPageNum) -> Self {
    Self {
      vpn,
      dealloc: false,
      panic: false,
    }
  }

  #[allow(unused)]
  pub fn with_vpn(mut self, vpn: VirtPageNum) -> Self {
    self.vpn = vpn;
    self
  }

  pub fn with_dealloc(mut self, dealloc: bool) -> Self {
    self.dealloc = dealloc;
    self
  }

  pub fn with_panic(mut self, panic: bool) -> Self {
    self.panic = panic;
    self
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

  pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
    self.find_ppn(va.clone().floor()).map(|ppn| {
      (PhysAddr::from(ppn).0 + va.page_offset()).into()
    })
  }

  pub fn map(&mut self, args: MapArgs) {
    let MapArgs { vpn, ppn, flags, mut frame } = args;
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

  pub fn unmap(&mut self, args: UnmapArgs) {
    let vpn = args.vpn;
    let pte = match self.find_pte(vpn) {
      Some(pte) if pte.is_valid() => pte,
      _ => if args.panic {
        panic!("vpn {:?} should mapped but not", vpn);
      } else {
        return;
      }
    };
    *pte = PageTableEntry::empty();
    if args.dealloc {
      // TODO: release record in self.frames
      let dummy_key = FrameTrackerMarker::new(pte.ppn());
      let key_to_remove = dummy_key.frame_tracker_ref();
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

pub fn translated_byte_buffer(
  page_table_token: usize,
  va_ptr: *const u8,
  len: usize,
) -> Vec<&'static [u8]> {
  let page_table = PageTable::from_token(page_table_token);
  let mut len_to_find = len;
  let mut cur_va = va_ptr as usize;
  let mut ret = Vec::with_capacity(len / PAGE_SIZE + 1);
  while len_to_find > 0 {
    let va = VirtAddr::from(cur_va);
    // TODO: fix malicious input
    let ppn = page_table.find_ppn(va.floor()).unwrap();
    let cur_len = PAGE_SIZE.min(len_to_find.min(PAGE_SIZE - va.page_offset()));
    ret.push(&ppn.get_bytes_array()[va.page_offset()..va.page_offset() + cur_len]);
    len_to_find -= cur_len;
    cur_va += cur_len;
  }
  ret
}

pub fn translate_str(page_table_token: usize, va_ptr: *const u8) -> String {
  let page_table = PageTable::from_token(page_table_token);
  let mut ret = String::new();
  let mut va = va_ptr as usize;
  // TODO: performance & security
  loop {
    let ch = *(page_table.translate_va(va.into())).unwrap().get_mut::<u8>();
    if ch == 0 {
      break;
    } else {
      ret.push(ch as char);
      va += 1;
    }
  }
  ret
}
