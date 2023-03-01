/// enables honoring the no execute bit on pages
pub fn enable_nxe() {
    use crate::x86_64::registers::msr::{rdmsr, wrmsr, IA32_EFER};

    let nxe_bit = 1 << 11;
    unsafe {
        let efer = rdmsr(IA32_EFER);
        wrmsr(IA32_EFER, efer | nxe_bit);
    }
}

/// Enables write protection on pages in kernel mode code
pub fn enable_write_protect() {
    use crate::x86_64::registers::control_regs::{cr0, cr0_write, Cr0};

    unsafe { cr0_write(cr0() | Cr0::WRITE_PROTECT) }
}