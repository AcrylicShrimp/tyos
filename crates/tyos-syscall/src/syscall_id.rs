pub struct SyscallId;

impl SyscallId {
    pub const SHUTDOWN: usize = 0x0001;
    pub const EXIT: usize = 0x0002;
    pub const GROW_HEAP: usize = 0x0010;
    pub const READ: usize = 0x0100;
    pub const WRITE: usize = 0x0101;
    pub const SPAWN: usize = 0x1000;
    pub const WAIT: usize = 0x1001;
}
