.section .text

.global _reset_handler

.extern kmain

_vectors:
    .long 0x00100000
    .long _reset_handler

    .rept 45
        .long _default_handler
    .endr

    .long _trap15_handler


_reset_handler:
    move.l #0x40010000, %sp

    move.l #0x40000000, %d0
    movec %d0, %vbr

    jsr kmain
    stop #0x2700


_default_handler:
    stop #0x2700
    bra.s _default_handler


_trap15_handler:
    rte
