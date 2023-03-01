#![allow(unused_variables)]

// export submodules
pub mod heap_allocator;
pub mod paging;

// re-exports
pub use self::paging::kernel_remap;

// imports
use self::paging::PhysicalAddress;
use core::sync::atomic::{AtomicBool, Ordering};
use crate::memory;
use multiboot2::{BootInformation, MemoryArea, MemoryAreaIter};
use crate::x86_64::instructions::memory as x86mem;

// TODO: This file needs to be refactored into sub modules

pub const PAGE_SIZE: u64 = 4096;

// Debuging toggles
pub const PRINT_DETAILED_KSYMS: bool = false;
pub const FRAME_ALLOC_TEST: bool = false;
pub const PAGING_TEST: bool = false;

static INIT_CALLED: AtomicBool = AtomicBool::new(false);

pub fn init(mb_info: &BootInformation) {
    // make sure init() is only called once...this will panic but thats better than tainting the kernel.
    assert!(!INIT_CALLED.load(Ordering::Relaxed));
    INIT_CALLED.store(true, Ordering::Relaxed);

    use self::paging::Page;
    use {memory::heap_allocator::HEAP_SIZE, memory::heap_allocator::HEAP_START};

    let memory_map_tag = mb_info.memory_map_tag().expect("Memory map tag required");

    println!("Memory areas from Multiboot2:");
    for area in memory_map_tag.memory_areas() {
        println!(
            "    start: 0x{:x}, length: 0x{:x} kbytes: {} ",
            area.start_address(),
            area.size(),
            area.size() / 1024
        );
    }

    let elf_sections_tag = mb_info
        .elf_sections_tag()
        .expect("Elf-sections tag required");

    if PRINT_DETAILED_KSYMS {
        println!(
            "{} Kernel sections loaded",
            elf_sections_tag.sections().count()
        );
        for section in elf_sections_tag.sections() {
            println!(
                "    name: {} addr: 0x{:x}, size: 0x{:x}, flags: 0x{:x}",
                section.name(),
                section.start_address(),
                section.size(),
                section.flags()
            );
        }
    }

    let kernel_start = elf_sections_tag
        .sections()
        .filter(|s| s.is_allocated())
        .map(|s| s.start_address())
        .min()
        .unwrap();
    let kernel_end = elf_sections_tag
        .sections()
        .filter(|s| s.is_allocated())
        .map(|s| s.start_address() + s.size())
        .max()
        .unwrap();

    println!(
        "kernel start: {:#x}, kernel end: {:#x}",
        kernel_start, kernel_end
    );
    println!(
        "multiboot start: {:#x}, multiboot end: {:#x}",
        mb_info.start_address(),
        mb_info.end_address()
    );

    let mut frame_allocator = memory::AreaFrameAllocator::new(
        kernel_start,
        kernel_end,
        mb_info.start_address() as u64,
        mb_info.end_address() as u64,
        memory_map_tag.memory_areas(),
    );

    if FRAME_ALLOC_TEST {
        println!(
            "FrameAllocator test: {} frames allocated",
            frame_allocator.alloc_test()
        );
    }

    if PAGING_TEST {
        memory::paging::paging_test(&mut frame_allocator);
    }

    x86mem::enable_nxe();
    let mut active_table = memory::kernel_remap(&mut frame_allocator, &mb_info);
    x86mem::enable_write_protect();

    // map heap
    let heap_start_page = Page::containing_address(HEAP_START as u64);
    let heap_end_page = Page::containing_address((HEAP_START + HEAP_SIZE - 1) as u64);

    for page in Page::range_inclusive(heap_start_page, heap_end_page) {
        active_table.map(
            page,
            paging::entry::EntryFlags::WRITABLE,
            &mut frame_allocator,
        );
    }

    println!("Initial kernel heap @ {:#x}, size={}", HEAP_START, HEAP_SIZE/1024);
}

struct FrameIter {
    start: Frame,
    end: Frame,
}

impl Iterator for FrameIter {
    type Item = Frame;

    fn next(&mut self) -> Option<Frame> {
        if self.start <= self.end {
            let frame = self.start.clone();
            self.start.number += 1;
            Some(frame)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frame {
    number: u64,
}

impl Frame {
    fn containing_addr(address: u64) -> Frame {
        Frame {
            number: address / PAGE_SIZE,
        }
    }

    fn start_address(&self) -> PhysicalAddress {
        self.number * PAGE_SIZE
    }

    // Private by design, a FrameAllocator is the only thing that can make new frames
    // we also do not implement the Clone trait on purpose.
    fn clone(&self) -> Frame {
        Frame {
            number: self.number,
        }
    }

    fn range_inclusive(start: Frame, end: Frame) -> FrameIter {
        FrameIter {
            start,
            end,
        }
    }
}

pub trait FrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame>;
    fn deallocate_frame(&mut self, frame: Frame);
}

pub struct AreaFrameAllocator {
    next_free_frame: Frame,
    current_area: Option<&'static MemoryArea>,
    areas: MemoryAreaIter,
    kernel_start: Frame,
    kernel_end: Frame,
    multiboot_start: Frame,
    multiboot_end: Frame,
}

impl AreaFrameAllocator {
    fn choose_next_area(&mut self) {
        self.current_area = self
            .areas
            .clone()
            .filter(|area| {
                let address = area.start_address() + area.size() - 1;
                Frame::containing_addr(address as u64) >= self.next_free_frame
            })
            .min_by_key(|area| area.start_address());

        if let Some(area) = self.current_area {
            let start_frame = Frame::containing_addr(area.start_address() as u64);
            if self.next_free_frame < start_frame {
                self.next_free_frame = start_frame;
            }
        }
    }

    pub fn new(
        kernel_start: u64,
        kernel_end: u64,
        multiboot_start: u64,
        multiboot_end: u64,
        memory_areas: MemoryAreaIter,
    ) -> AreaFrameAllocator {
        let mut allocator = AreaFrameAllocator {
            next_free_frame: Frame::containing_addr(0),
            current_area: None,
            areas: memory_areas,
            kernel_start: Frame::containing_addr(kernel_start),
            kernel_end: Frame::containing_addr(kernel_end),
            multiboot_start: Frame::containing_addr(multiboot_start),
            multiboot_end: Frame::containing_addr(multiboot_end),
        };

        allocator.choose_next_area();
        allocator
    }

    pub fn alloc_test(&mut self) -> usize {
        let mut frames: usize = 0;
        for i in 0.. {
            if self.allocate_frame().is_none() {
                frames = i;
                break;
            }
        }
        frames
    }
}

impl FrameAllocator for AreaFrameAllocator {
    fn allocate_frame(&mut self) -> Option<Frame> {
        if let Some(area) = self.current_area {
            // "Clone" the frame to return it if it's free. Frame doesn't
            // implement Clone, but we can construct an identical frame.
            let frame = Frame {
                number: self.next_free_frame.number,
            };

            // the last frame of the current area
            let current_area_last_frame = {
                let address = area.start_address() + area.size() - 1;
                Frame::containing_addr(address as u64)
            };

            if frame > current_area_last_frame {
                // all frames of current area are used, switch to next area
                self.choose_next_area();
            } else if frame >= self.kernel_start && frame <= self.kernel_end {
                // `frame` is used by the kernel
                self.next_free_frame = Frame {
                    number: self.kernel_end.number + 1,
                };
            } else if frame >= self.multiboot_start && frame <= self.multiboot_end {
                // `frame` is used by the multiboot information structure
                self.next_free_frame = Frame {
                    number: self.multiboot_end.number + 1,
                };
            } else {
                // frame is unused, increment `next_free_frame` and return it
                self.next_free_frame.number += 1;
                return Some(frame);
            }

            // `frame` was not valid, try it again with the updated `next_free_frame`
            self.allocate_frame()
        } else {
            None // no free frames left
        }
    }

    fn deallocate_frame(&mut self, frame: Frame) {
        // TODO: frame deallocation
        unimplemented!();
    }
}
