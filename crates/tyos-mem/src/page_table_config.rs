#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PageTableMode {
    Bare = 0,
    Sv39 = 8,
    Sv48 = 9,
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableConfig {
    /// The value of the `satp` register of the RISC-V architecture.
    ///
    /// Layout:
    /// ```text
    /// 63      60 59                  44 43                                0
    /// +--------+-----------------------+----------------------------------+
    /// |  MODE  |         ASID          |               PPN                |
    /// +--------+-----------------------+----------------------------------+
    ///   4 bits          16 bits                      44 bits
    /// ```
    config: usize,
}

impl PageTableConfig {
    pub fn new(mode: PageTableMode, asid: usize, ppn: usize) -> Self {
        Self {
            config: (mode as usize) << 60 | (asid << 44) | ppn,
        }
    }

    pub fn config(self) -> usize {
        self.config
    }
}
