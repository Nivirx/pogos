//! Instructions for loading descriptor tables (GDT, IDT, etc.).

use crate::x86_64::structures::gdt::SegmentSelector;
use crate::x86_64::structures::DescriptorTablePointer;

/// Load GDT table.
pub unsafe fn lgdt(gdt: &DescriptorTablePointer) {
    llvm_asm!("lgdt ($0)" :: "r" (gdt) : "memory");
}

/// Load LDT table.
pub unsafe fn lldt(ldt: &DescriptorTablePointer) {
    llvm_asm!("lldt ($0)" :: "r" (ldt) : "memory");
}

/// Load IDT table.
pub unsafe fn lidt(idt: &DescriptorTablePointer) {
    llvm_asm!("lidt ($0)" :: "r" (idt) : "memory");
}

/// Load the task state register using the `ltr` instruction.
pub unsafe fn load_tss(sel: SegmentSelector) {
    llvm_asm!("ltr $0" :: "r" (sel.0));
}
