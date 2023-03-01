use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use core::ptr;

pub const HEAP_START: usize = 0x4000_0000;
pub const HEAP_SIZE: usize = 32 * (1024 * 1024); // 32MB

pub struct BCAlloc {
    #[allow(dead_code)]
    heap_start: usize,

    heap_end: usize,
    next: AtomicUsize,
}

impl BCAlloc {
    pub const fn new() -> Self {
        Self {
            heap_start: HEAP_START,
            heap_end: (HEAP_START + HEAP_SIZE),
            next: AtomicUsize::new(HEAP_START),
        }
    }

    /// Align upwards. Returns the smallest x with alignment `align`
    /// so that x >= addr. The alignment must be a power of 2.
    fn align_up(addr: usize, align: usize) -> usize {
        BCAlloc::align_down(addr + align - 1, align)
    }

    /// Align downwards. Returns the greatest x with alignment `align`
    /// so that x <= addr. The alignment must be a power of 2.
    fn align_down(addr: usize, align: usize) -> usize {
        if align.is_power_of_two() {
            addr & !(align - 1)
        } else if align == 0 {
            addr
        } else {
            panic!("`align` must be a power of 2");
        }
    }
}

unsafe impl GlobalAlloc for BCAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        loop {
            let this_next = self.next.load(Ordering::Relaxed);
            let alloc_start = BCAlloc::align_up(this_next, layout.align());
            let alloc_end = alloc_start.saturating_add(layout.size());

            if alloc_end <= self.heap_end {
                let new_next = self
                    .next
                    .compare_and_swap(this_next, alloc_end, Ordering::Relaxed);
                if new_next == this_next {
                    return this_next as *mut u8;
                }
            } else {
                // heap exhuasted
                return ptr::null_mut(); // return null?
            }
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // Leak memory for now
        println!("[FIXME] Leaked {} bytes!", layout.size());
    }
}
