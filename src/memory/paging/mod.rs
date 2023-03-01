pub mod entry;
mod mapper;
pub mod table;
mod temporary_page;

pub use self::entry::*;
pub use self::mapper::Mapper;
use self::temporary_page::TemporaryPage;
use core::ops::{Deref, DerefMut};
use crate::memory::{Frame, FrameAllocator, PAGE_SIZE};
use multiboot2::BootInformation;

const ENTRY_COUNT: u64 = 512;

pub type PhysicalAddress = u64;
pub type VirtualAddress = u64;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Page {
    number: u64,
}

impl Page {
    pub fn containing_address(address: VirtualAddress) -> Page {
        assert!(
            address < 0x0000_8000_0000_0000 || address >= 0xffff_8000_0000_0000,
            "Invalid address in page translation: 0x{:x}",
            address
        );
        Page {
            number: address / PAGE_SIZE,
        }
    }

    pub fn start_address(self) -> u64 {
        self.number * PAGE_SIZE
    }

    fn p4_index(self) -> u64 {
        (self.number >> 27) & 0o777
    }

    fn p3_index(self) -> u64 {
        (self.number >> 18) & 0o777
    }

    fn p2_index(self) -> u64 {
        (self.number >> 9) & 0o777
    }

    fn p1_index(self) -> u64 {
        self.number & 0o777
    }

    pub fn range_inclusive(start: Page, end: Page) -> PageIter {
        PageIter {
            start,
            end
        }
    }
}

pub struct PageIter {
    start: Page,
    end: Page,
}

impl Iterator for PageIter {
    type Item = Page;

    fn next(&mut self) -> Option<Page> {
        if self.start <= self.end {
            let page = self.start;
            self.start.number += 1;
            Some(page)
        } else {
            None
        }
    }
}

pub fn paging_test<A>(alloc: &mut A)
where
    A: FrameAllocator,
{
    let mut page_table = unsafe { ActivePageTable::new() };

    let addr = 42 * 512 * 512 * 4096; // 42nd P3 entry 0xa_8000_0000
    let page = Page::containing_address(addr);
    let frame = alloc.allocate_frame().expect("no more frames");
    let frame2 = Frame {
        number: frame.number,
    };

    println!(
        "None = {:?}, map to {:?}",
        page_table.translate(addr),
        frame
    );
    page_table.map_to(page, frame, EntryFlags::empty(), alloc);
    println!("Some = {:?}", page_table.translate(addr));
    println!("next free frame: {:?}\n", alloc.allocate_frame());

    println!("First 32 bytes from mapped frame");
    // read first 4 qword from maped page (should be random)
    for i in 0..4 {
        println!(
            "frame: {} {:#x}@{:#x} = {:#x}",
            &frame2.number,
            &frame2.start_address(),
            Page::containing_address(addr).start_address() + (i * 8),
            unsafe { *((Page::containing_address(addr).start_address() + (i * 8)) as *const u64) }
        );
    }

    // unmap
    page_table.unmap(Page::containing_address(addr), alloc);
    println!("None = {:?}", page_table.translate(addr));
}

pub struct ActivePageTable {
    mapper: Mapper,
}

impl Deref for ActivePageTable {
    type Target = Mapper;

    fn deref(&self) -> &Mapper {
        &self.mapper
    }
}

impl DerefMut for ActivePageTable {
    fn deref_mut(&mut self) -> &mut Mapper {
        &mut self.mapper
    }
}

impl ActivePageTable {
    unsafe fn new() -> ActivePageTable {
        ActivePageTable {
            mapper: Mapper::new(),
        }
    }

    pub fn with<F>(
        &mut self,
        table: &mut InactivePageTable,
        temp_page: &mut temporary_page::TemporaryPage,
        f: F,
    ) where
        F: FnOnce(&mut Mapper),
    {
        use crate::x86_64::instructions::tlb;
        use crate::x86_64::registers::control_regs;
        {
            // backup p4
            let p4_backup = Frame::containing_addr(control_regs::cr3().0);
            let p4_table = temp_page.map_table_frame(p4_backup.clone(), self);

            // rewrite recursive map for active_p4[511] -> inactive_p4[0]
            self.p4_mut()[511].set(
                table.p4_frame.clone(),
                EntryFlags::PRESENT | EntryFlags::WRITABLE,
            );
            tlb::flush_all();

            // execute f in new context
            f(self);

            // restore p4 recursive map
            p4_table[511].set(p4_backup, EntryFlags::PRESENT | EntryFlags::WRITABLE);
            tlb::flush_all();
        }

        // unmap temp page outside of scope where it is used
        temp_page.unmap(self);
    }

    pub fn switch(&mut self, new_table: InactivePageTable) -> InactivePageTable {
        use crate::x86_64::registers::control_regs;
        use crate::x86_64::PhysicalAddress;

        let old_table = InactivePageTable {
            p4_frame: Frame::containing_addr(control_regs::cr3().0),
        };

        unsafe {
            control_regs::cr3_write(PhysicalAddress(new_table.p4_frame.start_address() as u64));
        }
        old_table
    }
}

pub struct InactivePageTable {
    p4_frame: Frame,
}

impl InactivePageTable {
    pub fn new(
        frame: Frame,
        active_table: &mut ActivePageTable,
        temp_page: &mut TemporaryPage,
    ) -> InactivePageTable {
        // inner scope is required to ensure that table is out of scope before unmaping
        {
            let table = temp_page.map_table_frame(frame.clone(), active_table);
            // zero table to clear random data from fetched frame
            table.zero();
            // configure recursive mapping for the table
            table[511].set(frame.clone(), EntryFlags::PRESENT | EntryFlags::WRITABLE)
        }

        temp_page.unmap(active_table);
        InactivePageTable { p4_frame: frame }
    }
}

pub fn kernel_remap<A>(allocator: &mut A, boot_info: &BootInformation) -> ActivePageTable
where
    A: FrameAllocator,
{
    let mut temp_page = TemporaryPage::new(Page { number: 0xDEAD_BEEF }, allocator);
    let mut active_table = unsafe { ActivePageTable::new() };
    let mut new_table = {
        let frame = allocator
            .allocate_frame()
            .expect("no free frames availible!");
        InactivePageTable::new(frame, &mut active_table, &mut temp_page)
    };

    active_table.with(&mut new_table, &mut temp_page, |mapper| {
        let elf_sections_tag = boot_info
            .elf_sections_tag()
            .expect("[Multiboot2] Memory map tag required");

        for section in elf_sections_tag.sections() {
            if !section.is_allocated() {
                // section is not loaded to memory
                continue;
            }
            assert!(
                section.start_address() % PAGE_SIZE as u64 == 0,
                "sections need to be page aligned"
            );

            println!(
                "mapping {} section at address: {:#x}, size: {:#x}",
                section.name(),
                section.start_address(),
                section.size()
            );

            let flags = EntryFlags::from_elf_section_flags(&section);

            let start_frame = Frame::containing_addr(section.start_address());
            let end_frame = Frame::containing_addr(section.end_address() - 1);

            // map sections in kernel elf
            for frame in Frame::range_inclusive(start_frame, end_frame) {
                mapper.identity_map(frame, flags, allocator);
            }
        }

        // map vga buffer
        let vga_buffer_frame = Frame::containing_addr(0xb8000);
        mapper.identity_map(vga_buffer_frame, EntryFlags::WRITABLE, allocator);

        // map multiboot header
        let mb2_start = Frame::containing_addr(boot_info.start_address() as u64);
        let mb2_end = Frame::containing_addr((boot_info.end_address() - 1) as u64);

        for f in Frame::range_inclusive(mb2_start, mb2_end) {
            mapper.identity_map(f, EntryFlags::WRITABLE, allocator);
        }
    });

    let old_table = active_table.switch(new_table);
    println!("Kernel remaped!");

    // create a guard page from the old p4 table that was used for kernel init
    let old_p4_page = Page::containing_address(old_table.p4_frame.start_address());
    active_table.unmap(old_p4_page, allocator); // unmap the page so an access causes a page fault

    // The page guard is mapped to the top of the old tables so p2/p3 become additional stack space
    // i.e .bss will be 32kb p4
    // p4 (guard page) = 0x1000
    // p3 = 0x1000
    // p2 = 0x1000
    // p1
    println!("guard page setup at {:#x}", old_p4_page.start_address());

    active_table
}
