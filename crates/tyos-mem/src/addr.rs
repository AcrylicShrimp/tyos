pub const V2P_ADDR_OFFSET: usize = 0xFFFF_FFC0_0000_0000;
pub const K2P_ADDR_OFFSET: usize = 0xFFFF_FFFF_0000_0000;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VAddr {
    pub addr: usize,
}

impl VAddr {
    pub const NULL: Self = Self { addr: 0 };

    pub fn from_raw_addr(addr: usize) -> Self {
        Self { addr }
    }

    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self { addr: ptr as usize }
    }

    pub fn to_ptr<T>(self) -> *const T {
        self.addr as *const T
    }

    pub fn to_mut_ptr<T>(self) -> *mut T {
        self.addr as *mut T
    }

    pub fn ppn2(self) -> usize {
        (self.addr >> 30) & 0x1FF
    }

    pub fn ppn1(self) -> usize {
        (self.addr >> 21) & 0x1FF
    }

    pub fn ppn0(self) -> usize {
        (self.addr >> 12) & 0x1FF
    }

    pub fn offset(self) -> usize {
        self.addr & 0xFFF
    }

    pub fn align_up(self, alignment: usize) -> Self {
        Self {
            addr: (self.addr + alignment - 1) & !(alignment - 1),
        }
    }

    pub fn align_down(self, alignment: usize) -> Self {
        Self {
            addr: self.addr & !(alignment - 1),
        }
    }

    pub fn to_hhdm(self) -> Self {
        Self::from_raw_addr(self.addr - (K2P_ADDR_OFFSET - V2P_ADDR_OFFSET))
    }

    pub fn to_kernel(self) -> Self {
        Self::from_raw_addr(self.addr + (K2P_ADDR_OFFSET - V2P_ADDR_OFFSET))
    }

    pub fn from_hhdm_to_physical(self) -> PAddr {
        PAddr::from_raw_addr(self.addr - V2P_ADDR_OFFSET)
    }

    pub fn from_kernel_to_physical(self) -> PAddr {
        PAddr::from_raw_addr(self.addr - K2P_ADDR_OFFSET)
    }
}

impl core::ops::Add<usize> for VAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            addr: self.addr + rhs,
        }
    }
}

impl core::ops::Add<VAddr> for usize {
    type Output = VAddr;

    fn add(self, rhs: VAddr) -> Self::Output {
        Self::Output {
            addr: rhs.addr + self,
        }
    }
}

impl core::ops::Sub<usize> for VAddr {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            addr: self.addr - rhs,
        }
    }
}

impl core::ops::Sub<VAddr> for VAddr {
    type Output = usize;

    fn sub(self, rhs: VAddr) -> Self::Output {
        self.addr - rhs.addr
    }
}

impl core::ops::AddAssign<usize> for VAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.addr += rhs;
    }
}

impl core::ops::SubAssign<usize> for VAddr {
    fn sub_assign(&mut self, rhs: usize) {
        self.addr -= rhs;
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PAddr {
    pub addr: usize,
}

impl PAddr {
    pub const NULL: Self = Self { addr: 0 };

    pub fn from_raw_addr(addr: usize) -> Self {
        Self { addr }
    }

    pub fn from_ptr<T>(ptr: *const T) -> Self {
        Self { addr: ptr as usize }
    }

    pub fn to_ptr<T>(self) -> *const T {
        self.addr as *const T
    }

    pub fn to_mut_ptr<T>(self) -> *mut T {
        self.addr as *mut T
    }

    pub fn ppn(self) -> usize {
        (self.addr >> 12) & 0x0000_03FF_FFFF_FFFF
    }

    pub fn align_up(self, alignment: usize) -> Self {
        Self {
            addr: (self.addr + alignment - 1) & !(alignment - 1),
        }
    }

    pub fn align_down(self, alignment: usize) -> Self {
        Self {
            addr: self.addr & !(alignment - 1),
        }
    }

    pub fn to_hhdm(self) -> VAddr {
        VAddr::from_raw_addr(self.addr + V2P_ADDR_OFFSET)
    }

    pub fn to_kernel(self) -> VAddr {
        VAddr::from_raw_addr(self.addr + K2P_ADDR_OFFSET)
    }
}

impl core::ops::Add<usize> for PAddr {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            addr: self.addr + rhs,
        }
    }
}

impl core::ops::Add<PAddr> for usize {
    type Output = PAddr;

    fn add(self, rhs: PAddr) -> Self::Output {
        Self::Output {
            addr: rhs.addr + self,
        }
    }
}

impl core::ops::Sub<usize> for PAddr {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            addr: self.addr - rhs,
        }
    }
}

impl core::ops::Sub<PAddr> for PAddr {
    type Output = usize;

    fn sub(self, rhs: PAddr) -> Self::Output {
        self.addr - rhs.addr
    }
}

impl core::ops::AddAssign<usize> for PAddr {
    fn add_assign(&mut self, rhs: usize) {
        self.addr += rhs;
    }
}

impl core::ops::SubAssign<usize> for PAddr {
    fn sub_assign(&mut self, rhs: usize) {
        self.addr -= rhs;
    }
}
