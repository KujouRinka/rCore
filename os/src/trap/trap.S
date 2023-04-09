.altmacro
.macro SAVE_GP n
    sd x\n, \n*8(sp)
.endm
.macro LOAD_GP n
    ld x\n, \n*8(sp)
.endm
    .section .text
    .globl __alltraps
    .align 2
__alltraps:
    # swap sp and sscratch
    csrrw sp, sscratch, sp
    # MAY REMOVE THIS INSTRUCTION
    addi sp, sp, -34*8
    sd x1, 1*8(sp)
    sd x3, 3*8(sp)
    .set n, 5
    .rept 27
        SAVE_GP %n
        .set n, n+1
    .endr
    # save sstatus and secp
    csrr t0, sstatus
    csrr t1, sepc
    sd t0, 32*8(sp)
    sd t1, 33*8(sp)
    # save user stack x2
    csrr t2, sscratch
    sd t2, 2*8(sp)
    mv a0, sp
    call trap_handler

    .globl __restore
__restore:
    # mv sp, a0
    # read sstatus
    ld t0, 32*8(sp)
    # read sepc
    ld t1, 33*8(sp)
    # read user stack addr
    ld t2, 2*8(sp)
    csrw sstatus, t0
    csrw sepc, t1
    csrw sscratch, t2
    ld x1, 1*8(sp)
    ld x3, 3*8(sp)
    .set n, 5
    .rept 27
        LOAD_GP %n
        .set n, n+1
    .endr
    # MAY REMOVE THIS INSTRUCTION
    addi sp, sp, 34*8
    csrrw sp, sscratch, sp
    sret
