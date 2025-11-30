use crate::timer_interrupt_handler::timer_interrupt_handler;
use tyos_uart::println;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapContext {
    pub regs: [usize; 31],
    pub sstatus: usize,
    pub sepc: usize,
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn trap_all() {
    core::arch::naked_asm!(
        "addi sp, sp, -272",
        "",
        "sd x1, 0 * 8(sp)",
        "sd x2, 1 * 8(sp)",
        "sd x3, 2 * 8(sp)",
        "sd x4, 3 * 8(sp)",
        "sd x5, 4 * 8(sp)",
        "sd x6, 5 * 8(sp)",
        "sd x7, 6 * 8(sp)",
        "sd x8, 7 * 8(sp)",
        "sd x9, 8 * 8(sp)",
        "sd x10, 9 * 8(sp)",
        "sd x11, 10 * 8(sp)",
        "sd x12, 11 * 8(sp)",
        "sd x13, 12 * 8(sp)",
        "sd x14, 13 * 8(sp)",
        "sd x15, 14 * 8(sp)",
        "sd x16, 15 * 8(sp)",
        "sd x17, 16 * 8(sp)",
        "sd x18, 17 * 8(sp)",
        "sd x19, 18 * 8(sp)",
        "sd x20, 19 * 8(sp)",
        "sd x21, 20 * 8(sp)",
        "sd x22, 21 * 8(sp)",
        "sd x23, 22 * 8(sp)",
        "sd x24, 23 * 8(sp)",
        "sd x25, 24 * 8(sp)",
        "sd x26, 25 * 8(sp)",
        "sd x27, 26 * 8(sp)",
        "sd x28, 27 * 8(sp)",
        "sd x29, 28 * 8(sp)",
        "sd x30, 29 * 8(sp)",
        "sd x31, 30 * 8(sp)",
        "",
        "csrr t0, sstatus",
        "sd t0, 31 * 8(sp)",
        "",
        "csrr t1, sepc",
        "sd t1, 32 * 8(sp)",
        "",
        "mv a0, sp",
        "call trap_handler",
        "",
        "ld t1, 32 * 8(sp)",
        "csrw sepc, t1",
        "",
        "ld t0, 31 * 8(sp)",
        "csrw sstatus, t0",
        "",
        "ld x1, 0 * 8(sp)",
        "ld x3, 2 * 8(sp)",
        "ld x4, 3 * 8(sp)",
        "ld x5, 4 * 8(sp)",
        "ld x6, 5 * 8(sp)",
        "ld x7, 6 * 8(sp)",
        "ld x8, 7 * 8(sp)",
        "ld x9, 8 * 8(sp)",
        "ld x10, 9 * 8(sp)",
        "ld x11, 10 * 8(sp)",
        "ld x12, 11 * 8(sp)",
        "ld x13, 12 * 8(sp)",
        "ld x14, 13 * 8(sp)",
        "ld x15, 14 * 8(sp)",
        "ld x16, 15 * 8(sp)",
        "ld x17, 16 * 8(sp)",
        "ld x18, 17 * 8(sp)",
        "ld x19, 18 * 8(sp)",
        "ld x20, 19 * 8(sp)",
        "ld x21, 20 * 8(sp)",
        "ld x22, 21 * 8(sp)",
        "ld x23, 22 * 8(sp)",
        "ld x24, 23 * 8(sp)",
        "ld x25, 24 * 8(sp)",
        "ld x26, 25 * 8(sp)",
        "ld x27, 26 * 8(sp)",
        "ld x28, 27 * 8(sp)",
        "ld x29, 28 * 8(sp)",
        "ld x30, 29 * 8(sp)",
        "ld x31, 30 * 8(sp)",
        "",
        "ld x2, 1 * 8(sp)",
        "addi sp, sp, 272",
        "",
        "sret",
    )
}

#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(ctx: &mut TrapContext) {
    let scause: usize;
    let stval: usize;

    unsafe {
        core::arch::asm!("csrr {}, scause", out(reg) scause);
        core::arch::asm!("csrr {}, stval", out(reg) stval);
    }

    let is_interrupt = (scause >> 63) != 0;
    let code = scause & !(1 << 63);

    if is_interrupt {
        match code {
            5 => {
                timer_interrupt_handler();
            }
            _ => {
                println!("[INTERRUPT] unknown interrupt: {code}");
            }
        }
    } else {
        match code {
            8 => {
                println!("[EXCEPTION] syscall from user mode");
                ctx.sepc += 4;
            }
            12 => {
                panic!("[EXCEPTION] instruction page fault at 0x{stval:x}");
            }
            13 | 15 => {
                panic!("[EXCEPTION] page fault at 0x{stval:x}");
            }
            _ => {
                panic!(
                    "[EXCEPTION] unhandled exception: code={code}, val={stval:x}, epc={:x}",
                    ctx.sepc,
                );
            }
        }
    }
}
