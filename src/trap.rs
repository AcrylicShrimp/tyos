use crate::timer_interrupt_handler::timer_interrupt_handler;
use tyos_uart::println;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapContext {
    pub regs: [usize; 30],
    pub sstatus: usize,
    pub sepc: usize,
    pub sscratch: usize,
}

#[unsafe(naked)]
#[unsafe(no_mangle)]
pub extern "C" fn trap_all() {
    const TRAP_FRAME_SIZE: usize = size_of::<TrapContext>();

    core::arch::naked_asm!(
        // switch to kernel stack
        "csrrw sp, sscratch, sp",
        "bnez sp, trap",
        "",
        // sp can be zero if the trap is from kernel
        // switch back to kernel stack if sp is zero
        "csrrw sp, sscratch, sp",
        "",
        "trap:",
        // create trap frame
        "addi sp, sp, -{TRAP_FRAME_SIZE}",
        "",
        // save general registers
        "sd x1, 0 * 8(sp)",
        // x2 is sp; never save it
        "sd x3, 1 * 8(sp)",
        "sd x4, 2 * 8(sp)",
        "sd x5, 3 * 8(sp)",
        "sd x6, 4 * 8(sp)",
        "sd x7, 5 * 8(sp)",
        "sd x8, 6 * 8(sp)",
        "sd x9, 7 * 8(sp)",
        "sd x10, 8 * 8(sp)",
        "sd x11, 9 * 8(sp)",
        "sd x12, 10 * 8(sp)",
        "sd x13, 11 * 8(sp)",
        "sd x14, 12 * 8(sp)",
        "sd x15, 13 * 8(sp)",
        "sd x16, 14 * 8(sp)",
        "sd x17, 15 * 8(sp)",
        "sd x18, 16 * 8(sp)",
        "sd x19, 17 * 8(sp)",
        "sd x20, 18 * 8(sp)",
        "sd x21, 19 * 8(sp)",
        "sd x22, 20 * 8(sp)",
        "sd x23, 21 * 8(sp)",
        "sd x24, 22 * 8(sp)",
        "sd x25, 23 * 8(sp)",
        "sd x26, 24 * 8(sp)",
        "sd x27, 25 * 8(sp)",
        "sd x28, 26 * 8(sp)",
        "sd x29, 27 * 8(sp)",
        "sd x30, 28 * 8(sp)",
        "sd x31, 29 * 8(sp)",
        "",
        // save sstatus, sepc, and sscratch
        "csrr t0, sstatus",
        "sd t0, 30 * 8(sp)",
        "",
        "csrr t1, sepc",
        "sd t1, 31 * 8(sp)",
        "",
        "csrr t2, sscratch",
        "sd t2, 32 * 8(sp)",
        "",
        // clear sscratch, marking the stack as kernel stack
        "csrw sscratch, x0",
        "",
        // call trap handler
        "mv a0, sp",
        "call trap_handler",
        "",
        // restore sscratch, sepc, and sstatus
        "ld t2, 32 * 8(sp)",
        "csrw sscratch, t2",
        "",
        "ld t1, 31 * 8(sp)",
        "csrw sepc, t1",
        "",
        "ld t0, 30 * 8(sp)",
        "csrw sstatus, t0",
        "",
        // restore general registers
        "ld x1, 0 * 8(sp)",
        // x2 is sp; never restore it
        "ld x3, 1 * 8(sp)",
        "ld x4, 2 * 8(sp)",
        // x5 is t0;
        // t0 is currently holding the sstatus value;
        // we need to test the sstatus value to check if spp is zero before returning from trap,
        // so delay the restore
        "ld x6, 4 * 8(sp)",
        "ld x7, 5 * 8(sp)",
        "ld x8, 6 * 8(sp)",
        "ld x9, 7 * 8(sp)",
        "ld x10, 8 * 8(sp)",
        "ld x11, 9 * 8(sp)",
        "ld x12, 10 * 8(sp)",
        "ld x13, 11 * 8(sp)",
        "ld x14, 12 * 8(sp)",
        "ld x15, 13 * 8(sp)",
        "ld x16, 14 * 8(sp)",
        "ld x17, 15 * 8(sp)",
        "ld x18, 16 * 8(sp)",
        "ld x19, 17 * 8(sp)",
        "ld x20, 18 * 8(sp)",
        "ld x21, 19 * 8(sp)",
        "ld x22, 20 * 8(sp)",
        "ld x23, 21 * 8(sp)",
        "ld x24, 22 * 8(sp)",
        "ld x25, 23 * 8(sp)",
        "ld x26, 24 * 8(sp)",
        "ld x27, 25 * 8(sp)",
        "ld x28, 26 * 8(sp)",
        "ld x29, 27 * 8(sp)",
        "ld x30, 28 * 8(sp)",
        "ld x31, 29 * 8(sp)",
        "",
        // switch back to kernel stack
        "andi t0, t0, 1 << 8", // check if spp is zero
        "bnez t0, kernel_exit", // jump to kernel_exit if spp is non-zero
        "",
        "user_exit:",
        // restore t0 from kernel stack
        "ld x5, 3 * 8(sp)",
        // drop trap frame
        "addi sp, sp, {TRAP_FRAME_SIZE}",
        // switch back to kernel stack
        "csrrw sp, sscratch, sp",
        // return from trap
        "sret",
        "",
        "kernel_exit:",
        // restore t0 from kernel stack
        "ld x5, 3 * 8(sp)",
        // drop trap frame
        "addi sp, sp, {TRAP_FRAME_SIZE}",
        // return from trap
        "sret",
        TRAP_FRAME_SIZE = const TRAP_FRAME_SIZE,
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
