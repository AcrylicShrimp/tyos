.section .text

.global _start

.extern kmain

_start:
    move.l #0x10000, %sp
    jsr kmain
    stop #0x2700
