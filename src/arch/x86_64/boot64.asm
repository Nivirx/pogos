bits 64
extern rust_main

section .text
global start64
start64:
    ; load 0 into all data segments
    mov ax, 0
    mov ss, ax
    mov dx, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    pop rdi             ; multiboot pointer
    call rust_main

    ; print OKAY
    mov rax, 0x2f592f412f4b2f4f
    mov qword [0xb8000], rax
    hlt

section .text
global _heap_oom
_heap_oom:
    mov rax, 0x4f21_4f4d_4f4f_4f4f
    mov qword [0xb8000], rax
    hlt