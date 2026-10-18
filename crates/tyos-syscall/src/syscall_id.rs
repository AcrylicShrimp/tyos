pub struct SyscallId;

impl SyscallId {
    pub const EXIT: usize = 0x0001;
    pub const GROW_HEAP: usize = 0x0010;
    pub const READ: usize = 0x0100;
    pub const WRITE: usize = 0x0101;
}
