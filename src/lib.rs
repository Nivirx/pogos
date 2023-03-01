#![no_std]
#![no_main]

#![feature(lang_items)]
#![feature(const_fn)]
#![feature(ptr_internals)]
#![feature(allocator_api)]
#![feature(integer_atomics)]
#![feature(panic_info_message)]
#![feature(asm)]
#![feature(abi_x86_interrupt)]
#![feature(box_syntax)]
#![feature(llvm_asm)]
#![feature(const_fn_trait_bound)]

#![allow(unused_attributes)]
#![allow(unused_parens)]


#[macro_use]
extern crate alloc;
//extern crate rlibc;
extern crate spin;
extern crate volatile;
extern crate bit_field;

#[macro_use]
extern crate bitflags;

extern crate multiboot2;
extern crate raw_cpuid;
extern crate lazy_static;

pub mod x86_64;

#[macro_use]
pub mod serial;

pub mod memory;

#[macro_use]
pub mod vga_buffer;

// external crates in scope
use core::panic::PanicInfo;
use raw_cpuid::CpuId;

// internal crates in scope
use memory::heap_allocator::BCAlloc;

// This is not safe for multithreaded heap allocations
#[global_allocator]
static KALLOC: BCAlloc = BCAlloc::new();

#[no_mangle]
pub extern "C" fn rust_main(mb2_header: u32) {
    vga_buffer::clear_screen();
    vga_println!("Booting from multiboot, kernel messages on serial port 0x3F8");
    vga_println!("Kernel entry point @ {:p}", rust_main as *const ());

    println!();
    println!("Booting in x64 long mode from multiboot...");

    let boot_info = unsafe { multiboot2::load(mb2_header as usize) };
    memory::init(&boot_info);

    let cpuid = CpuId::new();
    match cpuid.get_vendor_info() {
        Some(vf) => println!("CPU Vendor: {}", vf.as_string()),
        None => (),
    };
    match cpuid.get_extended_function_info() {
        Some(efn) => println!("CPU: {}", efn.processor_brand_string().expect("UNK")),
        None => (),
    }
    /* TODO: maybe more infomation about cacheline or cache topology here */

    heap_test();
    
    
    // jump to real rust main

    // returns back to start function (start64)
}

fn heap_test() {
    println!("Heap test running");

    // test out the heap
    use alloc::boxed::Box;
    let mut heap_test: Box<usize> = Box::new(0xDEAD_BEEF);
    *heap_test -= 0xBEEF;
    let heap_test2 = Box::new("hello");
    println!("{:#x} {:?}", *heap_test, *heap_test2);

    let mut vec_test = vec![1, 2, 3, 4, 5, 6, 7];
    vec_test[3] = 42;
    for i in &vec_test {
        print!("{} ", i);
    }
    println!();

    // allocate a lot of memory
    println!("testing large heap allocations - 16KB");
    let _heap_test3 = box [0x2A as u8; (0x400 << 4)];

    println!("testing large heap allocations - 64KB");
    let _heap_test3 = box [0x0F as u8; (0x400 << 6)];

    println!("testing large heap allocations - 256KB");
    let _heap_test3 = box [0xF0 as u8; (0x400 << 8)];

    println!("testing large heap allocations - 1MB");
    let _heap_test3 = box [0x2B as u8; (0x400 << 10)];

    println!("testing large heap allocations - 2MB");
    let _heap_test4 = box [0xFF as u8; (0x400 << 11)];

    println!("testing large heap allocations - 4MB");
    let _heap_test5 = box [0xBC as u8; (0x400 << 12)];

    println!("testing large heap allocations - 8MB");
    let _heap_test5 = box [0xFE as u8; (0x400 << 13)];

    println!("testing large heap allocations - 16MB");
    let _heap_test5 = box [0xFD as u8; (0x400 << 14)];

    println!("Heap test completed");
}

/// enable no execute bit in EFER register


/// calling this function should result in a page fault
/// if it does not page fault the LLVM used by rustc does not have stack probes support
fn __test_stack_probes() {
    let _boom = [0; 99999];
}

// RLS sees the below as errors because I think RLS parses the std crate by default
// no idea how to stop that...

#[lang = "eh_personality"] extern fn eh_personality() {}

#[lang = "panic_impl"]
#[no_mangle]
pub extern "C" fn kernel_panic(info: &PanicInfo) -> ! {
    println!("!!! [OOPS] !!!\n\nPANIC at {:?}", info.location());
    println!("    {:#?}", info.message());
    println!("!!! [OOPS] !!!");
    loop {}
}

#[lang = "oom"]
#[no_mangle]
pub fn heap_alloc_oom(_: core::alloc::Layout) -> ! {
    extern "C" {
        fn _heap_oom() -> !;
    }
    unsafe { _heap_oom() }
}