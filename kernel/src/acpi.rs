use crate::global::Global;
use core::ops::Index;
// ref https://wiki.osdev.org/RSDP
// ref MikanOS list 11.27 (p285)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Rsdp {
    pub signature: [u8; 8],
    pub checksum: u8,
    pub oemid: [u8; 6],
    pub revision: u8,
    pub rsdt_address: u32,
    pub length: u32,
    pub xsdt_address: u64,
    pub extended_checksum: u8,
    pub reserved: [u8; 3],
}

// ref MikanOS list11.28-
impl Rsdp {
    pub fn is_valid(&self) -> bool {
        // 構造体をバイト列として扱う
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const Rsdp as *const u8,
                core::mem::size_of::<Rsdp>(),
            )
        };

        // 1.Signatureが "RSD PTR " であることを確認
        if &bytes[0..8] != b"RSD PTR " {
            return false;
        }

        // 2. revisionが2であるか
        if self.revision != 2 {
            return false;
        }

        // 3. 先頭20バイトの合計が 0(mod 256)かどうか
        let sum20: u8 = bytes[..20].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if sum20 != 0 {
            return false;
        }

        // 4.先頭36バイトの合計が 0(mod 256)かどうか
        let sum36 = bytes[..36].iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if sum36 != 0 {
            return false;
        }

        return true;
    }
}
pub fn initialize(rsdp: &Rsdp) {
    if !rsdp.is_valid() {
        panic!("RSDP is not valid");
    }

    let xsdt = unsafe { &*(rsdp.xsdt_address as *const Xsdt) };
    if !xsdt.header.is_valid(b"XSDT") {
        panic!("XSDT is not valid");
    }

    for i in 0..xsdt.count() {
        let entry = &xsdt[i];
        if entry.is_valid(b"FACP") {
            FADT.with(|fadt| {
                *fadt = Some(unsafe { &*(entry as *const DescriptionHeader as *const Fadt) });
            });
            break;
        }
    }
    FADT.with(|fadt| {
        if fadt.is_none() {
            panic!("FADT is not found");
        }
    });
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct DescriptionHeader {
    signature: [u8; 4],
    length: u32,
    revision: u8,
    checksum: u8,
    oemid: [u8; 6],
    oem_table_id: [u8; 8],
    oem_revision: u32,
    creator_id: u32,
    creator_revision: u32,
}

impl DescriptionHeader {
    pub fn is_valid(&self, expected_signature: &[u8; 4]) -> bool {
        if &self.signature != expected_signature {
            return false;
        }
        let bytes = unsafe {
            core::slice::from_raw_parts(
                self as *const DescriptionHeader as *const u8,
                self.length as usize,
            )
        };
        let sum: u8 = bytes.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
        if sum != 0 {
            return false;
        }
        return true;
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Xsdt {
    header: DescriptionHeader,
}

impl Xsdt {
    pub fn count(&self) -> usize {
        (self.header.length as usize - core::mem::size_of::<DescriptionHeader>())
            / core::mem::size_of::<u64>()
    }
}
impl Index<usize> for Xsdt {
    type Output = DescriptionHeader;
    fn index(&self, index: usize) -> &Self::Output {
        let entries = (self as *const Xsdt as *const u8)
            .wrapping_add(core::mem::size_of::<DescriptionHeader>())
            as *const u64;
        let entry_addr = unsafe { *entries.add(index) };
        unsafe { &*(entry_addr as *const DescriptionHeader) }
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct Fadt {
    header: DescriptionHeader,
    reserved1: [u8; 76 - size_of::<DescriptionHeader>()],
    pm_timer_block: u32,
    reserved2: [u8; 112 - 80],
    flags: u32,
    reserved3: [u8; 276 - 116],
}

static FADT: Global<Option<&Fadt>> = Global::new(None);
