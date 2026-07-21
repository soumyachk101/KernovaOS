//! Paging: OffsetPageTable over the bootloader's full physical-memory mapping,
//! plus a frame allocator over the BootInfo memory map.

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, OffsetPageTable, Page, PageTable, PhysFrame, Size4KiB,
};
use x86_64::{PhysAddr, VirtAddr};

/// Kernel-global mapper + frame allocator + phys offset, set once by
/// `init_globals` from kernel_main. Later subsystems (user address spaces at
/// M11) allocate frames through these instead of threading locals around.
pub static MAPPER: Mutex<Option<OffsetPageTable<'static>>> = Mutex::new(None);
pub static FRAME_ALLOCATOR: Mutex<Option<BootInfoFrameAllocator>> = Mutex::new(None);
pub static PHYS_OFFSET: Mutex<Option<VirtAddr>> = Mutex::new(None);

/// # Safety
/// Same contracts as [`init`] + [`BootInfoFrameAllocator::init`]; call once.
pub unsafe fn init_globals(physical_memory_offset: VirtAddr, memory_map: &'static MemoryMap) {
    *MAPPER.lock() = Some(init(physical_memory_offset));
    *FRAME_ALLOCATOR.lock() = Some(BootInfoFrameAllocator::init(memory_map));
    *PHYS_OFFSET.lock() = Some(physical_memory_offset);
}

/// Initialize an OffsetPageTable for the active level-4 table.
///
/// # Safety
/// Caller must guarantee all physical memory is mapped at
/// `physical_memory_offset` (bootloader `map_physical_memory` feature) and
/// that this is called only once (aliasing `&mut PageTable` otherwise).
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// # Safety
/// Same contract as [`init`].
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    // SAFETY: CR3 points at the active PML4 frame; with the full physical
    // mapping, `virt` is a valid unique mapping of it (uniqueness per the
    // call-once contract above).
    &mut *page_table_ptr
}

/// Maps `page` to the VGA text frame 0xB8000 — demo/test helper.
pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    // SAFETY: demo-only: maps the VGA frame, which is already mapped
    // elsewhere — acceptable aliasing because both mappings treat it as
    // volatile device memory (no Rust references are formed to it).
    let map_to_result = unsafe { mapper.map_to(page, frame, flags, frame_allocator) };
    map_to_result.expect("map_to failed").flush();
}

/// Frame allocator returning usable frames from the bootloader memory map.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// # Safety
    /// Caller must guarantee the memory map is valid and its `Usable` frames
    /// are really unused (true for the map the bootloader hands us).
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        self.memory_map
            .iter()
            .filter(|r| r.region_type == MemoryRegionType::Usable)
            .map(|r| r.range.start_addr()..r.range.end_addr())
            .flat_map(|r| r.step_by(4096))
            .map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

// SAFETY: usable_frames yields only frames the bootloader marked Usable, and
// `next` ensures each frame is returned at most once.
unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // ponytail: O(n²) re-iteration per allocation; fine until a real
        // bitmap/stack allocator is needed
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}
