pub const EI_NIDENT: usize = 16;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Ehdr {
    pub e_ident: [u8; EI_NIDENT],
    pub e_type: u16,
    pub e_machine: u16,
    pub e_version: u32,
    pub e_entry: u64,
    pub e_phoff: u64,
    pub e_shoff: u64,
    pub e_flags: u32,
    pub e_ehsize: u16,
    pub e_phentsize: u16,
    pub e_phnum: u16,
    pub e_shentsize: u16,
    pub e_shnum: u16,
    pub e_shstrndx: u16,
}

// ELF64 の値のうち、ローダが使うものだけを定義している。
// (ET_REL / ET_DYN / PF_* / PT_DYNAMIC などは必要になった時に足す)
/// 実行可能ファイル (e_type)
pub const ET_EXEC: u16 = 2;
/// ロード対象のセグメント (p_type)
pub const PT_LOAD: u32 = 1;

use crate::error::{KernelError, Result};

impl Elf64Ehdr {
    /// バイト列から ELF64 ヘッダを安全にパースする。
    pub fn parse(file: &[u8]) -> Result<Self> {
        if file.len() < core::mem::size_of::<Self>() {
            return Err(KernelError::InvalidFormat);
        }
        if &file[0..4] != b"\x7fELF" {
            return Err(KernelError::InvalidFormat);
        }
        // SAFETY: サイズおよびマジックナンバーの検査済み。
        // read_unaligned によりアライメント違反を防ぐ。
        let ehdr = unsafe { core::ptr::read_unaligned(file.as_ptr() as *const Self) };
        Ok(ehdr)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Elf64Phdr {
    pub p_type: u32,
    pub p_flags: u32,
    pub p_offset: u64,
    pub p_vaddr: u64,
    pub p_paddr: u64,
    pub p_filesz: u64,
    pub p_memsz: u64,
    pub p_align: u64,
}

impl Elf64Phdr {
    /// セグメントがファイル内で保持するデータ領域のスライスを返す。
    pub fn file_image<'a>(&self, file: &'a [u8]) -> Result<&'a [u8]> {
        let start = self.p_offset as usize;
        let end = start
            .checked_add(self.p_filesz as usize)
            .ok_or(KernelError::InvalidFormat)?;
        file.get(start..end).ok_or(KernelError::InvalidFormat)
    }
}

/// ELF ファイル内の LOAD セグメントを走査するイテレータを返す。
pub fn load_segments<'a>(
    file: &'a [u8],
    ehdr: &'a Elf64Ehdr,
) -> impl Iterator<Item = Elf64Phdr> + 'a {
    (0..ehdr.e_phnum as usize).filter_map(move |i| {
        let phdr_size = core::mem::size_of::<Elf64Phdr>();
        let offset = (ehdr.e_phoff as usize).checked_add(i.checked_mul(phdr_size)?)?;
        let end = offset.checked_add(phdr_size)?;
        if end > file.len() {
            return None;
        }
        let phdr = unsafe {
            core::ptr::read_unaligned(file[offset..].as_ptr() as *const Elf64Phdr)
        };
        if phdr.p_type == PT_LOAD {
            Some(phdr)
        } else {
            None
        }
    })
}
