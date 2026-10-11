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
}
