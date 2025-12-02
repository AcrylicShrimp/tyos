OUTPUT_ARCH(riscv)
ENTRY(_start)

BASE_PADDR = 0x80200000;
BASE_VADDR = 0xFFFFFFFF80200000;

SECTIONS
{
  . = BASE_VADDR;

  _kernel_start = .;

  .text : AT(ADDR(.text) - BASE_VADDR + BASE_PADDR) {
    *(.text.init)
    *(.text .text.*)
  }

  .rodata : AT(ADDR(.rodata) - BASE_VADDR + BASE_PADDR) {
    *(.rodata .rodata.*)
  }

  .data : AT(ADDR(.data) - BASE_VADDR + BASE_PADDR) {
    *(.data .data.*)
  }

  .bss : AT(ADDR(.bss) - BASE_VADDR + BASE_PADDR) {
    _bss_start = .;
    *(.bss .bss.*)
    *(COMMON)
    _bss_end = .;
  }

  .pgtable : AT(ADDR(.pgtable) - BASE_VADDR + BASE_PADDR) {
    . = ALIGN(4096);
    *(.pgtable)
  }

  . = ALIGN(4096);
  _stack_bottom = .;
  . += 128 * 1024; 
  _stack_top = .;

  . = ALIGN(4096);
  _kernel_end = .;
  _heap_start = .;
}
