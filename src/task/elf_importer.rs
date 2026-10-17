use crate::task::{
    loader::{LoadedProgram, ProgramLoadError, load_program},
    program::{
        ProgramDefConfig, ProgramDefSection, ProgramDefSectionPermission, ProgramDefSource,
        builder::{ProgramBuildError, ProgramDefBuilder},
    },
};
use goblin::{elf::Elf, elf64::program_header, error::Error as GoblinError};
use tyos_mem::addr::VAddr;

#[derive(Debug)]
pub enum ElfImportError {
    FileSizeExceedsMemSize { file_size: usize, mem_size: usize },
    GoblinError(GoblinError),
    ProgramBuildError(ProgramBuildError),
    ProgramLoadError(ProgramLoadError),
}

impl From<GoblinError> for ElfImportError {
    fn from(err: GoblinError) -> Self {
        ElfImportError::GoblinError(err)
    }
}

impl From<ProgramBuildError> for ElfImportError {
    fn from(value: ProgramBuildError) -> Self {
        ElfImportError::ProgramBuildError(value)
    }
}

impl From<ProgramLoadError> for ElfImportError {
    fn from(err: ProgramLoadError) -> Self {
        ElfImportError::ProgramLoadError(err)
    }
}

pub fn import_elf(
    elf_file: &[u8],
    config: &ProgramDefConfig,
) -> Result<LoadedProgram, ElfImportError> {
    let elf = Elf::parse(elf_file)?;
    let mut program_def = ProgramDefBuilder::new(VAddr::from_raw_addr(elf.entry as usize));

    for header in &elf.program_headers {
        if header.p_type != program_header::PT_LOAD {
            continue;
        }

        let vaddr = VAddr::from_raw_addr(header.p_vaddr as usize);
        let mem_size = header.p_memsz as usize;
        let file_offset = header.p_offset as usize;
        let file_size = header.p_filesz as usize;

        if file_size > mem_size {
            return Err(ElfImportError::FileSizeExceedsMemSize {
                file_size,
                mem_size,
            });
        }

        if mem_size == 0 {
            continue;
        }

        program_def = program_def.add_section(ProgramDefSection {
            addr: vaddr,
            size: mem_size,
            permission: permission_to_section_permission(header.p_flags),
        })?;

        if file_size > 0 {
            program_def = program_def.add_source(ProgramDefSource {
                dest_addr: vaddr,
                src_offset: file_offset,
                len: file_size,
            })?;
        }
    }

    let program_def = program_def.build(config, elf_file.len())?;
    let loaded_program = load_program(&program_def, elf_file)?;

    Ok(loaded_program)
}

fn permission_to_section_permission(flags: u32) -> ProgramDefSectionPermission {
    let mut permission = ProgramDefSectionPermission::NONE;

    if flags & program_header::PF_R != 0 {
        permission |= ProgramDefSectionPermission::READ;
    }

    if flags & program_header::PF_W != 0 {
        permission |= ProgramDefSectionPermission::WRITE;
    }

    if flags & program_header::PF_X != 0 {
        permission |= ProgramDefSectionPermission::EXEC;
    }

    permission
}
