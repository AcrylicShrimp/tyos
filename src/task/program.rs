use alloc::vec::Vec;
use core::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Range};
use tyos_mem::addr::VAddr;

#[derive(Debug, Clone)]
pub struct ProgramDef {
    entry: VAddr,
    /// A list of program sections.
    sections: Vec<ProgramDefSection>,
    /// A list of program sources.
    sources: Vec<ProgramDefSource>,
}

impl ProgramDef {
    pub fn entry(&self) -> VAddr {
        self.entry
    }

    /// Lists all the sections ordered by it's start address.
    pub fn sections(&self) -> &[ProgramDefSection] {
        &self.sections
    }

    /// Lists all the sources ordered by it's destination start address.
    pub fn sources(&self) -> &[ProgramDefSource] {
        &self.sources
    }

    pub fn end(&self) -> Option<VAddr> {
        self.sections.last().and_then(|section| section.end())
    }
}

#[derive(Debug, Clone)]
pub struct ProgramDefSection {
    pub addr: VAddr,
    pub size: usize,
    pub permission: ProgramDefSectionPermission,
}

impl ProgramDefSection {
    pub fn start(&self) -> VAddr {
        self.addr
    }

    pub fn end(&self) -> Option<VAddr> {
        self.addr
            .addr
            .checked_add(self.size)
            .map(VAddr::from_raw_addr)
    }

    fn overlaps(lhs: &Self, rhs: &Self) -> bool {
        let lhs_end = match lhs.end() {
            Some(end) => end,
            None => {
                return false;
            }
        };
        let rhs_end = match rhs.end() {
            Some(end) => end,
            None => {
                return false;
            }
        };

        lhs.start() < rhs_end && rhs.start() < lhs_end
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProgramDefSectionPermission(usize);

impl ProgramDefSectionPermission {
    pub const NONE: Self = Self(0);

    /// Readable Section
    pub const READ: Self = Self(1 << 1);

    /// Writable Section
    ///
    /// It implies `READ`.
    pub const WRITE: Self = Self(1 << 2);

    /// Executable Section
    pub const EXEC: Self = Self(1 << 3);
}

impl ProgramDefSectionPermission {
    pub fn bits(self) -> usize {
        self.0
    }

    pub fn contains(self, flags: Self) -> bool {
        self.0 & flags.0 == flags.0
    }
}

impl BitAnd for ProgramDefSectionPermission {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl BitAndAssign for ProgramDefSectionPermission {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl BitOr for ProgramDefSectionPermission {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl BitOrAssign for ProgramDefSectionPermission {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Debug, Clone)]
pub struct ProgramDefSource {
    pub dest_addr: VAddr,
    pub src_offset: usize,
    pub len: usize,
}

impl ProgramDefSource {
    pub fn dest_start(&self) -> VAddr {
        self.dest_addr
    }

    pub fn dest_end(&self) -> Option<VAddr> {
        self.dest_addr
            .addr
            .checked_add(self.len)
            .map(VAddr::from_raw_addr)
    }

    pub fn src_start(&self) -> usize {
        self.src_offset
    }

    pub fn src_end(&self) -> Option<usize> {
        self.src_offset.checked_add(self.len)
    }

    fn dest_overlaps(lhs: &Self, rhs: &Self) -> bool {
        let lhs_dest_end = match lhs.dest_end() {
            Some(end) => end,
            None => {
                return false;
            }
        };
        let rhs_dest_end = match rhs.dest_end() {
            Some(end) => end,
            None => {
                return false;
            }
        };

        lhs.dest_start() < rhs_dest_end && rhs.dest_start() < lhs_dest_end
    }
}

#[derive(Debug, Clone)]
pub struct ProgramDefConfig {
    pub vmem_range: Range<VAddr>,
}

pub mod builder {
    use super::*;

    #[derive(Debug)]
    pub enum ProgramBuildError {
        InvalidConfig,
        InvalidSectionRange,
        InvalidSectionPermission,
        InvalidSourceDestRange,
        InvalidSourceSrcRange,
        NoSection,
        NoSource,
        EmptySection,
        EmptySource,
        SectionOverflow,
        SourceSrcOverflow,
        SourceDestOverflow,
        SectionOverlap,
        SourceDestOverlap,
        EntryOverflow,
        EntryNotInExecSection,
        EntryNotBackedBySource,
    }

    #[derive(Debug)]
    pub struct ProgramDefBuilder {
        entry: VAddr,
        /// A list of program sections ordered by the virtual address.
        sections: Vec<ProgramDefSection>,
        /// A list of program sources.
        sources: Vec<ProgramDefSource>,
    }

    impl ProgramDefBuilder {
        pub fn new(entry: VAddr) -> Self {
            Self {
                entry,
                sections: Vec::new(),
                sources: Vec::new(),
            }
        }

        pub fn add_section(
            mut self,
            section: ProgramDefSection,
        ) -> Result<Self, ProgramBuildError> {
            if section.end().is_none() {
                // the section must have a proper range defined
                return Err(ProgramBuildError::InvalidSectionRange);
            }

            if section.permission == ProgramDefSectionPermission::NONE {
                // the section must have at least one permission
                return Err(ProgramBuildError::InvalidSectionPermission);
            }

            self.sections.push(section);

            Ok(self)
        }

        pub fn add_source(mut self, source: ProgramDefSource) -> Result<Self, ProgramBuildError> {
            if source.dest_end().is_none() {
                // the source must have a proper dest range defined
                return Err(ProgramBuildError::InvalidSourceDestRange);
            }

            if source.src_end().is_none() {
                // the source must have a proper src range defined
                return Err(ProgramBuildError::InvalidSourceSrcRange);
            }

            self.sources.push(source);

            Ok(self)
        }

        pub fn build(
            mut self,
            config: &ProgramDefConfig,
            program_file_len: usize,
        ) -> Result<ProgramDef, ProgramBuildError> {
            if config.vmem_range.end <= config.vmem_range.start {
                // the virtual memory range must be sane
                return Err(ProgramBuildError::InvalidConfig);
            }

            if self.sections.is_empty() {
                // a valid program def must hold at least one section
                return Err(ProgramBuildError::NoSection);
            }

            if self.sources.is_empty() {
                // a valid program def must hold at least one source
                return Err(ProgramBuildError::NoSource);
            }

            if self.sections.iter().any(|section| section.size == 0) {
                // the sections must not be empty
                return Err(ProgramBuildError::EmptySection);
            }

            if self.sources.iter().any(|source| source.len == 0) {
                // the sources must not be empty
                return Err(ProgramBuildError::EmptySource);
            }

            if !self.sections.iter().all(|section| {
                config.vmem_range.start <= section.start()
                    && section.end().unwrap() <= config.vmem_range.end
            }) {
                // the sections must be within the virtual memory range
                return Err(ProgramBuildError::SectionOverflow);
            }

            if !self
                .sources
                .iter()
                .all(|source| source.src_end().unwrap() <= program_file_len)
            {
                // the sources must be within the program file size
                return Err(ProgramBuildError::SourceSrcOverflow);
            }

            if 2 <= self.sections.len() {
                self.sections.sort_unstable_by_key(|section| section.addr);

                for index in 0..(self.sections.len() - 1) {
                    let lhs = &self.sections[index];
                    let rhs = &self.sections[index + 1];

                    if ProgramDefSection::overlaps(lhs, rhs) {
                        // the sections must not be overlap
                        return Err(ProgramBuildError::SectionOverlap);
                    }
                }
            }

            if 2 <= self.sources.len() {
                self.sources.sort_unstable_by_key(|source| source.dest_addr);

                for index in 0..(self.sources.len() - 1) {
                    let lhs = &self.sources[index];
                    let rhs = &self.sources[index + 1];

                    if ProgramDefSource::dest_overlaps(lhs, rhs) {
                        // the source destinations must not be overlap
                        return Err(ProgramBuildError::SourceDestOverlap);
                    }
                }
            }

            for source in &self.sources {
                let section = match self
                    .sections
                    .binary_search_by_key(&source.dest_addr, |section| section.addr)
                {
                    Ok(index) => &self.sections[index],
                    Err(index) => match index {
                        0 => {
                            // the destination of the source overflows
                            return Err(ProgramBuildError::SourceDestOverflow);
                        }
                        index => &self.sections[index - 1],
                    },
                };

                if section.start() <= source.dest_start()
                    && source.dest_end().unwrap() <= section.end().unwrap()
                {
                    continue;
                }

                // the destination of the source overflows
                return Err(ProgramBuildError::SourceDestOverflow);
            }

            for section in &mut self.sections {
                if section
                    .permission
                    .contains(ProgramDefSectionPermission::WRITE)
                {
                    section.permission |= ProgramDefSectionPermission::READ;
                }
            }

            let entry_section = match self
                .sections
                .binary_search_by_key(&self.entry, |section| section.addr)
            {
                Ok(index) => &self.sections[index],
                Err(index) => match index {
                    0 => {
                        // the entry must be within a section
                        return Err(ProgramBuildError::EntryOverflow);
                    }
                    index => &self.sections[index - 1],
                },
            };

            if self.entry < entry_section.start() || entry_section.end().unwrap() <= self.entry {
                // the entry must be within a section
                return Err(ProgramBuildError::EntryOverflow);
            }

            if !entry_section
                .permission
                .contains(ProgramDefSectionPermission::EXEC)
            {
                // the entry must be within an executable section
                return Err(ProgramBuildError::EntryNotInExecSection);
            }

            let entry_source = match self
                .sources
                .binary_search_by_key(&self.entry, |source| source.dest_addr)
            {
                Ok(index) => &self.sources[index],
                Err(index) => match index {
                    0 => {
                        // the entry must be backed by a source
                        return Err(ProgramBuildError::EntryNotBackedBySource);
                    }
                    index => &self.sources[index - 1],
                },
            };

            if self.entry < entry_source.dest_start()
                || entry_source.dest_end().unwrap() <= self.entry
            {
                // the entry must be backed by a source
                return Err(ProgramBuildError::EntryNotBackedBySource);
            }

            Ok(ProgramDef {
                entry: self.entry,
                sections: self.sections,
                sources: self.sources,
            })
        }
    }
}
