use alloc::collections::BTreeMap;
use alloc::sync::Arc;
// use alloc::vec::Vec;
use core::arch::asm;
use core::ptr;
use lazy_static::lazy_static;
use log::{debug, trace};
use riscv::register::satp;
use crate::config::*;
use crate::mm::address::{PhysAddr, PhysPageNum, VirtAddr, VirtPageNum, VPNRange};
use crate::mm::frame_allocator::frame_alloc;
use crate::mm::page_table::{MapArgs, PageTable, PTEFlags, UnmapArgs};
use crate::mm::PageTableEntry;
use crate::sync::UPSafeCell;
use crate::vars::*;

lazy_static! {
  pub static ref KERNEL_SPACE: Arc<UPSafeCell<MemorySet>> =
    Arc::new(unsafe { UPSafeCell::new(MemorySet::new_kernel()) });
}

pub struct MemorySet {
  page_table: PageTable,
  areas: BTreeMap<VirtPageNum, MapArea>,
}

impl MemorySet {
  pub fn new_bare() -> Self {
    Self {
      page_table: PageTable::new(),
      areas: BTreeMap::new(),
    }
  }

  /// Map new kernel without kernel stack.
  /// # Kernel Layout:
  /// ```
  /// +-------------------+
  /// |  Physical Frames  |
  /// +-------------------+
  /// |      .bss         |
  /// +-------------------+
  /// |      .data        |
  /// +-------------------+
  /// |      .rodata      |
  /// +-------------------+
  /// |      .text        |
  /// +-------------------+  <- BASE_ADDRESS
  /// ```
  pub fn new_kernel() -> Self {
    let mut memory_set = Self::new_bare();
    // map trampoline
    memory_set.map_trampoline();
    // print out sections information
    debug!(".text [{:#x}, {:#x})", stext as usize, etext as usize);
    debug!(".rodata [{:#x}, {:#x})", srodata as usize, erodata as usize);
    debug!(".data [{:#x}, {:#x})", sdata as usize, edata as usize);
    debug!(".bss [{:#x}, {:#x})", sbss_with_stack as usize, ebss as usize);
    debug!("physical [{:#x}, {:#x})", ekernel as usize, MEMORY_END);
    // map .text section
    memory_set.push(MapArea::new(
      (stext as usize).into(),
      (etext as usize).into(),
      MapType::Identical,
      MapPermission::R | MapPermission::X,
    ), None);
    debug!("kernel.text section mapped");

    // map .rodata section
    memory_set.push(MapArea::new(
      (srodata as usize).into(),
      (erodata as usize).into(),
      MapType::Identical,
      MapPermission::R,
    ), None);
    debug!("kernel.rodata section mapped");

    // map .data section
    memory_set.push(MapArea::new(
      (sdata as usize).into(),
      (edata as usize).into(),
      MapType::Identical,
      MapPermission::R | MapPermission::W,
    ), None);
    debug!("kernel.data section mapped");

    // map .bss section
    memory_set.push(MapArea::new(
      (sbss_with_stack as usize).into(),
      (ebss as usize).into(),
      MapType::Identical,
      MapPermission::R | MapPermission::W,
    ), None);
    debug!("kernel.bss section mapped");

    // map physical memory
    memory_set.push(MapArea::new(
      (ekernel as usize).into(),
      MEMORY_END.into(),
      MapType::Identical,
      MapPermission::R | MapPermission::W,
    ), None);
    debug!("kernel.physical memory mapped");
    memory_set
  }

  /// Make [`MemorySet`] from elf file, with `user_stack_top` and `entry_point` return.
  /// # ELF Layout:
  /// ```
  /// High 256GB
  /// +-------------------+
  /// |  Trampoline Code  |
  /// +-------------------+  <- TRAMPOLINE
  /// |    TrapContext    |
  /// +-------------------+  <- TRAP_CONTEXT, user_stack_top
  /// |    User Stack     |
  /// +-------------------+  <- user_stack_bottom
  /// |       ...         |
  ///
  /// Low 256GB
  /// |       ...         |
  /// +-------------------+
  /// |    Heap Memory    |
  /// +-------------------+  <- heap_bottom
  /// |      .bss         |
  /// +-------------------+
  /// |      .data        |
  /// +-------------------+
  /// |      .rodata      |
  /// +-------------------+
  /// |      .text        |
  /// +-------------------+  <- BASE_ADDRESS (0x10000 va)
  pub fn from_elf(elf_data: &[u8]) -> (Self, usize, usize, usize) {
    let mut memory_set = Self::new_bare();
    memory_set.map_trampoline();
    let elf = xmas_elf::ElfFile::new(elf_data).unwrap();

    // map elf file at low address
    let elf_header = elf.header;
    let magic = elf_header.pt1.magic;
    assert_eq!(magic, [0x7f, 0x45, 0x4c, 0x46], "invalid elf!");
    let ph_count = elf_header.pt2.ph_count();
    let mut max_end_vpn = VirtPageNum(0);
    for i in 0..ph_count {
      let ph = elf.program_header(i).unwrap();
      if ph.get_type().unwrap() == xmas_elf::program::Type::Load {
        let start_va: VirtAddr = (ph.virtual_addr() as usize).into();
        let end_va: VirtAddr = ((ph.virtual_addr() + ph.mem_size()) as usize).into();
        let mut map_perm = MapPermission::U;
        let ph_flags = ph.flags();
        if ph_flags.is_read() { map_perm |= MapPermission::R; }
        if ph_flags.is_write() { map_perm |= MapPermission::W; }
        if ph_flags.is_execute() { map_perm |= MapPermission::X; }
        let map_area = MapArea::new(start_va, end_va, MapType::Framed, map_perm);
        max_end_vpn = max_end_vpn.max(map_area.vpn_range.get_end());
        memory_set.push(
          map_area,
          Some(&elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize]),
        );
      }
    }

    let heap_bottom: usize = VirtAddr::from(max_end_vpn).into();
    let user_stack_top = TRAP_CONTEXT;
    let user_stack_bottom = user_stack_top - USER_STACK_SIZE;

    // map user stack
    memory_set.push(MapArea::new(
      user_stack_bottom.into(),
      user_stack_top.into(),
      MapType::Framed,
      MapPermission::R | MapPermission::W | MapPermission::U,
    ), None);

    // map for sbrk
    memory_set.push(MapArea::new(
      heap_bottom.into(),
      heap_bottom.into(),
      MapType::Framed,
      MapPermission::R | MapPermission::W | MapPermission::U,
    ), None);

    // map TrapContext
    memory_set.push(MapArea::new(
      TRAP_CONTEXT.into(),
      TRAMPOLINE.into(),
      MapType::Framed,
      MapPermission::R | MapPermission::W,
    ), None);
    (
      memory_set,
      user_stack_top,
      heap_bottom,
      elf.header.pt2.entry_point() as usize
    )
  }

  pub fn from_another(another: &MemorySet) -> Self {
    // TODO: may do COW here
    let mut memory_set = Self::new_bare();
    memory_set.map_trampoline();
    for (start_vpn, ma) in another.areas.iter() {
      memory_set.areas.insert(start_vpn.clone(), ma.clone());
      memory_set.push(ma.clone(), None);
      for vpn in ma.vpn_range {
        let src_ppn = another.page_table.translate(vpn).unwrap().ppn();
        let dst_ppn = memory_set.page_table.translate(vpn).unwrap().ppn();
        dst_ppn.get_bytes_array()
          .copy_from_slice(src_ppn.get_bytes_array());
      }
    }
    memory_set
  }

  fn map_trampoline(&mut self) {
    self.page_table.map(
      MapArgs::builder(
        VirtAddr::from(TRAMPOLINE).into(),
        PhysAddr::from(strampoline as usize).into(),
      ).with_flags(PTEFlags::R | PTEFlags::X),
    );
  }
}

impl MemorySet {
  /// Insert VA to PTE
  /// # Safety
  /// VPNRange must not overlap with other areas.
  pub unsafe fn insert_framed_area(
    &mut self,
    start_va: VirtAddr,
    end_va: VirtAddr,
    permission: MapPermission,
  ) {
    self.push(MapArea::new(
      start_va,
      end_va,
      MapType::Framed,
      permission,
    ), None);
  }

  #[allow(unused)]
  pub fn remove_framed_area(
    &mut self,
    start_va: VirtAddr,
    end_va: VirtAddr,
  ) {
    for vpn in VPNRange::new(start_va.ceil(), end_va.floor()) {
      self.page_table.unmap(
        UnmapArgs::builder(vpn)
          .with_dealloc(true)
          .with_panic(false),
      );
      self.areas.remove(&vpn);
    }
  }

  pub fn remove_area_with_start_vpn(&mut self, start_vpn: VirtPageNum) {
    match self.areas.get_mut(&start_vpn) {
      Some(area) => {
        area.unmap(&mut self.page_table);
        self.areas.remove(&start_vpn);
      }
      None => return,
    }
  }

  pub fn activate(&self) {
    let satp = self.page_table.token();
    unsafe {
      satp::write(satp);
      asm!("sfence.vma");
    }
  }

  pub fn token(&self) -> usize {
    self.page_table.token()
  }

  pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
    self.page_table.translate(vpn)
  }

  #[allow(unused)]
  pub fn shrink_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
    if let Some(area) = self.areas.get_mut(&start.into()) {
      area.shrink_to(&mut self.page_table, new_end.ceil());
      true
    } else {
      false
    }
  }

  #[allow(unused)]
  pub fn append_to(&mut self, start: VirtAddr, new_end: VirtAddr) -> bool {
    if let Some(area) = self.areas.get_mut(&start.into()) {
      area.append_to(&mut self.page_table, new_end.ceil());
      true
    } else {
      false
    }
  }

  /// Insert [`MapArea`] to current address space.
  fn push(&mut self, mut map_area: MapArea, data: Option<&[u8]>) {
    map_area.map(&mut self.page_table);
    if let Some(data) = data {
      map_area.copy_data(&mut self.page_table, data);
    }
    self.areas.insert(map_area.vpn_range.get_start(), map_area);
  }
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum MapType {
  Identical,
  Framed,
}

bitflags! {
  pub struct MapPermission: u16 {
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

#[derive(Clone)]
struct MapArea {
  vpn_range: VPNRange,
  map_type: MapType,
  map_perm: MapPermission,
}

impl MapArea {
  fn new(
    start_va: VirtAddr,
    end_va: VirtAddr,
    map_type: MapType,
    map_perm: MapPermission,
  ) -> Self {
    let start = start_va.floor();
    let end = end_va.ceil();
    Self {
      vpn_range: VPNRange::new(start, end),
      map_type,
      map_perm,
    }
  }

  fn from_another(another: &MapArea) -> Self {
    another.clone()
  }

  /// Map `self.vpn_range` to specified [`PageTable`].
  fn map(&mut self, page_table: &mut PageTable) {
    for vpn in self.vpn_range {
      self.map_one(page_table, vpn);
    }
  }

  #[allow(unused)]
  /// Unmap `self.vpn_range` to specified [`PageTable`].
  fn unmap(&mut self, page_table: &mut PageTable) {
    for vpn in self.vpn_range {
      self.unmap_one(page_table, vpn);
    }
  }

  fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
    let (ppn, frame_tracker) = match self.map_type {
      MapType::Identical => {
        // Identical map has no need for allocating
        (PhysPageNum(vpn.0), None)
      }
      MapType::Framed => {
        // Framed map needs to alloc new page
        let frame = frame_alloc().unwrap();
        let ppn = frame.ppn;
        (ppn, Some(frame))
      }
    };
    let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
    page_table.map(
      MapArgs::builder(vpn, ppn)
        .with_flags(pte_flags)
        .with_frame(frame_tracker),
    );
  }

  fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
    let free = match self.map_type {
      MapType::Identical => false,
      MapType::Framed => true,
    };
    page_table.unmap(
      UnmapArgs::builder(vpn)
        .with_dealloc(free)
        .with_panic(true),
    );
  }

  fn shrink_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
    for vpn in VPNRange::new(new_end, self.vpn_range.get_end()) {
      self.unmap_one(page_table, vpn);
    }
    self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
  }

  fn append_to(&mut self, page_table: &mut PageTable, new_end: VirtPageNum) {
    for vpn in VPNRange::new(self.vpn_range.get_end(), new_end) {
      self.map_one(page_table, vpn);
    }
    self.vpn_range = VPNRange::new(self.vpn_range.get_start(), new_end);
  }

  /// Copy `data` to physical addr
  fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8]) {
    // assert_eq!(self.map_type, MapType::Framed);
    let mut start = 0usize;
    let mut len = data.len();
    for vpn in self.vpn_range {
      let to_write_bytes_len = len.min(PAGE_SIZE);
      unsafe {
        ptr::copy_nonoverlapping(
          data.as_ptr().add(start),
          page_table.find_ppn(vpn).unwrap().get_ptr_mut(),
          to_write_bytes_len,
        );
      }
      start += to_write_bytes_len;
      len -= to_write_bytes_len;
      if len == 0 {
        break;
      }
    }
  }
}

#[allow(unused)]
pub fn remap_test() {
  let mut kernel_space = KERNEL_SPACE.exclusive_access();
  let mid_text: VirtAddr = ((stext as usize + etext as usize) / 2).into();
  let mid_rodata: VirtAddr = ((srodata as usize + erodata as usize) / 2).into();
  let mid_data: VirtAddr = ((sdata as usize + edata as usize) / 2).into();
  assert_eq!(
    kernel_space.page_table.translate(mid_text.floor()).unwrap().is_writable(),
    false
  );
  assert_eq!(
    kernel_space.page_table.translate(mid_rodata.floor()).unwrap().is_writable(),
    false,
  );
  assert_eq!(
    kernel_space.page_table.translate(mid_data.floor()).unwrap().is_executable(),
    false,
  );
  // unmap test
  let (bottom, top): (VirtAddr, VirtAddr) =
    ((TRAP_CONTEXT - PAGE_SIZE).into(), TRAP_CONTEXT.into());
  kernel_space.push(MapArea::new(
    bottom,
    top,
    MapType::Framed,
    MapPermission::R,
  ), None);
  assert_eq!(
    kernel_space.page_table.translate(bottom.into()).unwrap().is_readable(),
    true,
  );
  kernel_space.page_table.unmap(
    UnmapArgs::builder(bottom.into())
      .with_dealloc(true)
      .with_panic(true),
  );
  assert_eq!(
    kernel_space.page_table.translate(bottom.into()).unwrap().is_readable(),
    false,
  );
  trace!("remap_test passed!");
}
