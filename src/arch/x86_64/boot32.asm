bits 32

global start32
extern start64

;;;
; Errors
;   0 = Multiboot2 error
;   1 = No CPUID support on this platform (VM or some very old hardware?)
;   2 = No Longmode support
;;;

section .text
start32:
    mov esp, stack_top

    push 0x0                ; write 0's for high dword of popq rdi later on in start64
    push ebx                ; preserve multiboot on stack

    ;mov edi, ebx           ; multiboot structure to edi for when rust_main() is called
                            ; this is a u32 in rust
                            
    call check_mb
    call check_cpuid
    call check_longmode

    call init_page_tables
    call enable_paging

    ; load 64-bit GDT
    lgdt [gdt64.pointer]

    ; far jump to long mode init
    jmp gdt64.code:start64

    mov dword [0xb8000], 0x2f4b2f4f
    hlt

; Early error printer. Prints 'ERR: X' where X is an error code in al
error:
    mov dword [0xb8000], 0x4f524f45
    mov dword [0xb8004], 0x4f3a4f52
    mov dword [0xb8008], 0x4f204f20
    mov byte  [0xb800a], al
    hlt

check_mb:
    cmp eax, 0x36d76289
    jne .no_mb
    ret
.no_mb:
    mov al, "0"
    jmp error

check_cpuid:
    ; flip cpuid id bit (bit 21) to check for cpuid support
    
    ; copy flags to eax via stack
    pushfd
    pop eax

    mov ecx, eax        ; copy to ecx for compare

    xor eax, 1 << 21    ; xor eax with the 21st bit set

    push eax            ; push new flags and pop to flags register
    popfd

    pushfd
    pop eax

    cmp eax, ecx        ; can we change the bit?
    je .no_cpuid
    ret
.no_cpuid:
    mov al, "1"
    jmp error

check_longmode:
    ; test for extended cpuid
    mov eax, 0x80000000
    cpuid
    cmp eax, 0x80000001
    jb .no_longmode

    mov eax, 0x80000001
    cpuid
    test edx, 1 << 29
    jz .no_longmode
    ret
.no_longmode:
    mov al, "2"
    jmp error

init_page_tables:
    ; map the 511th entry of P4 to P4 to setup recursive mapping later
    mov eax, p4_table
    or eax, 0b11    ; present + writable
    mov [p4_table + 511 * 8], eax
    
    ; map first P4 entry to P3 table
    mov eax, p3_table
    or eax, 0b11        ; present + writable
    mov [p4_table], eax

    ; map first P3 entry to P2 table
    mov eax, p2_table
    or eax, 0b11
    mov [p3_table], eax

    ; map each P2 entry to a 2MiB page
    mov ecx, 0
.map_p2_table:
    mov eax, 0x200000
    mul ecx
    or eax, 0b10000011      ; present + writable + huge
    mov [p2_table + ecx * 8], eax

    inc ecx
    cmp ecx, 512
    jne .map_p2_table

    ret

enable_paging:
    ; load P4 to cr3 register
    mov eax, p4_table
    mov cr3, eax

    ; enable PAE-flag in cr4
    mov eax, cr4
    or eax, 1 << 5
    mov cr4, eax

    ; set long mode bit in EFER MSR
    mov ecx, 0xC0000080
    rdmsr
    or eax, 1 << 8
    wrmsr

    ; enable paging in cr0 register
    mov eax, cr0
    or eax, 1 << 31
    mov cr0, eax

    ret

section .rodata
gdt64:
    dq 0 ; zero
.code: equ $ - gdt64
    dq (1<<43) | (1<<44) | (1<<47) | (1<<53) ; code segment
.pointer:
    dw $ - gdt64 - 1
    dq gdt64

section .bss
align 4096
p4_table:
    resb 4096           ; after the kernel is remapped this becomes the stack guard page
p3_table:
    resb 4096           ; after the kernel is remapped the original p3/p2 tables are not used
p2_table:               ; so they are used as an additional 8kb of stack space (24kb + 4k guard page total)
    resb 4096
stack_bottom:
    resb 4096 * 4       ; 16 KiB Stack
stack_top: