OUTPUT_ARCH(riscv)
ENTRY(_start)

SECTIONS
{
  /*
    RAM Start:               0x80000000
    + OpenSBI Firmware Size: 0x00200000
    = Kernel Load Address:   0x80200000
  */
  . = 0x80200000;

  _kernel_start = .;

  .text : {
    *(.text.init)
    *(.text .text.*)
  }

  .rodata : {
    *(.rodata .rodata.*)
  }

  .data : {
    *(.data .data.*)
  }

  .bss : {
    _bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    _bss_end = .;
  }
  
  . = ALIGN(4096);
  _stack_bottom = .;
  . += 128 * 1024; 
  _stack_top = .;

  . = ALIGN(4096);
  _kernel_end = .;
  _heap_start = .;
}
