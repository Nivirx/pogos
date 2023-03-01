//! Low level functions for special x86 instructions.
#![feature(llvm_asm)]
pub mod port;
pub mod interrupts;
pub mod tables;
pub mod tlb;
pub mod segmentation;
pub mod random;
pub mod memory;

/// Halts the CPU by executing the `hlt` instruction.
#[inline(always)]
pub unsafe fn halt() {
    llvm_asm!("hlt" :::: "volatile");
}

/// Read time stamp counters

/// Read the time stamp counter using the `RDTSC` instruction.
///
/// The `RDTSC` instruction is not a serializing instruction.
/// It does not necessarily wait until all previous instructions
/// have been executed before reading the counter. Similarly,
/// subsequent instructions may begin execution before the
/// read operation is performed. If software requires `RDTSC` to be
/// executed only after all previous instructions have completed locally,
/// it can either use `RDTSCP` or execute the sequence `LFENCE;RDTSC`.
pub fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        llvm_asm!("rdtsc" : "={eax}" (low), "={edx}" (high));
    }
    ((u64::from(high)) << 32) | (u64::from(low))
}

/// Read the time stamp counter using the `RDTSCP` instruction.
///
/// The `RDTSCP` instruction waits until all previous instructions
/// have been executed before reading the counter.
/// However, subsequent instructions may begin execution
/// before the read operation is performed.
///
/// Volatile is used here because the function may be used to act as
/// an instruction barrier.
pub fn rdtscp() -> u64 {
    let low: u32;
    let high: u32;
    unsafe {
        llvm_asm!("rdtscp" : "={eax}" (low), "={edx}" (high) ::: "volatile");
    }
    ((high as u64) << 32) | (low as u64)
}

// Model specific registers

/// Write 64 bits to msr register.
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    llvm_asm!("wrmsr" :: "{ecx}" (msr), "{eax}" (low), "{edx}" (high) : "memory" : "volatile" );
}

/// Read 64 bits msr register.
pub fn rdmsr(msr: u32) -> u64 {
    let (high, low): (u32, u32);
    unsafe {
        llvm_asm!("rdmsr" : "={eax}" (low), "={edx}" (high) : "{ecx}" (msr) : "memory" : "volatile");
    }
    ((high as u64) << 32) | (low as u64)
}
