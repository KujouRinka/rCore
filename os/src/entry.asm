    .attribute arch, "rv64gc"
    .section .text.entry
    .globl _start
_start:
    la sp, boot_stack_lower_bound
    # prepare stack for each core
    li t0, 4096
    mul t0, t0, a0
    add sp, sp, t0
    li t0, 4096
    add sp, sp, t0
    call rust_main

    .section .bss.stack
    .globl boot_stack_lower_bound
boot_stack_lower_bound:
    .space 4096 * 16
    .globl boot_stack_top
boot_stack_top: