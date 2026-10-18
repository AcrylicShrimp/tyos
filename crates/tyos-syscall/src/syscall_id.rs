pub struct SyscallId;

impl SyscallId {
    pub const READ: usize = 63;
    pub const WRITE: usize = 64;
    pub const EXIT: usize = 93;
}
