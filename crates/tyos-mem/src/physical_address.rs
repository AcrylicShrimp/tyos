#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct PhysicalAddress {
    pub addr: usize,
}

impl PhysicalAddress {
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
}

impl core::ops::Add<usize> for PhysicalAddress {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self {
            addr: self.addr + rhs,
        }
    }
}

impl core::ops::Add<PhysicalAddress> for usize {
    type Output = PhysicalAddress;

    fn add(self, rhs: PhysicalAddress) -> Self::Output {
        Self::Output {
            addr: rhs.addr + self,
        }
    }
}

impl core::ops::Sub<usize> for PhysicalAddress {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self {
            addr: self.addr - rhs,
        }
    }
}

impl core::ops::Sub<PhysicalAddress> for PhysicalAddress {
    type Output = usize;

    fn sub(self, rhs: PhysicalAddress) -> Self::Output {
        self.addr - rhs.addr
    }
}

impl core::ops::AddAssign<usize> for PhysicalAddress {
    fn add_assign(&mut self, rhs: usize) {
        self.addr += rhs;
    }
}

impl core::ops::SubAssign<usize> for PhysicalAddress {
    fn sub_assign(&mut self, rhs: usize) {
        self.addr -= rhs;
    }
}
