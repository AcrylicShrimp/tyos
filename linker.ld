ENTRY(_reset_handler)
SECTIONS
{
    . = 0x40000000;
    .text : { 
        */crt0.o(.text)
        *(.text)
    }
    .data : { *(.data) }
    .bss  : { *(.bss) }
}
