use crate::global::Global;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct Bpb {
    pub jump_boot: [u8; 3],
    pub oem_name: [u8; 8],
    pub bytes_per_sector: u16,
    pub sectors_per_cluster: u8,
    pub reserved_sector_count: u16,
    pub num_fats: u8,
    pub root_entry_count: u16,
    pub total_sectors_16: u16,
    pub media: u8,
    pub fat_size_16: u16,
    pub sectors_per_track: u16,
    pub num_heads: u16,
    pub hidden_sectors: u32,
    pub total_sectors_32: u32,
    pub fat_size_32: u32,
    pub ext_flags: u16,
    pub fs_version: u16,
    pub root_cluster: u32,
    pub fs_info: u16,
    pub backup_boot_sector: u16,
    pub reserved: [u8; 12],
    pub drive_number: u8,
    pub reserved1: u8,
    pub boot_signature: u8,
    pub volume_id: u32,
    pub volume_label: [u8; 11],
    pub fs_type: [u8; 8],
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Attribute {
    ReadOnly = 0x01,
    Hidden = 0x02,
    System = 0x04,
    VolumeId = 0x08,
    Directory = 0x10,
    Archive = 0x20,
    LongName = 0x0f,
}

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct DirectoryEntry {
    pub name: [u8; 11],
    pub attr: Attribute,
    pub ntres: u8,
    pub create_time_tenth: u8,
    pub create_time: u16,
    pub create_date: u16,
    pub last_access_date: u16,
    pub first_cluster_high: u16,
    pub write_time: u16,
    pub write_date: u16,
    pub first_cluster_low: u16,
    pub file_size: u32,
}

impl DirectoryEntry {
    pub fn first_cluster(&self) -> u32 {
        self.first_cluster_low as u32 | ((self.first_cluster_high as u32) << 16)
    }
}

pub static BOOT_VOLUME_IMAGE: Global<Option<usize>> = Global::new(None);

pub fn initialize(volume_image: *const u8) {
    BOOT_VOLUME_IMAGE.with(|bvi| *bvi = Some(volume_image as usize));
}

pub fn get_volume_image_addr() -> usize {
    BOOT_VOLUME_IMAGE.with(|bvi| bvi.expect("BOOT_VOLUME_IMAGE not initialized"))
}

pub fn boot_volume_image() -> Bpb {
    let addr = get_volume_image_addr();
    unsafe { core::ptr::read_unaligned(addr as *const Bpb) }
}

/// 指定されたクラスタ番号の先頭セクタが追いてあるメモリアドレスを返す
pub fn get_cluster_addr(cluster: usize) -> usize {
    let bpb = boot_volume_image();
    let sector_num = bpb.reserved_sector_count as usize
        + bpb.num_fats as usize * bpb.fat_size_32 as usize
        + (cluster - 2) * bpb.sectors_per_cluster as usize;
    let offset = sector_num * bpb.bytes_per_sector as usize;
    get_volume_image_addr() + offset
}

/// 指定されたクラスタの先頭セクタへのポインタを返す。
pub fn get_sector_by_cluster<T>(cluster: usize) -> *const T {
    get_cluster_addr(cluster) as *const T
}

/// ディレクトリエントリの短名を基本名(8byte)と拡張子(3byte)に分割して取得する。
/// パディングされた空白文字は 0 に置き換えられる。
pub fn read_name(entry: &DirectoryEntry, base: &mut [u8; 9], ext: &mut [u8; 4]) {
    let name = entry.name;

    base[..8].copy_from_slice(&name[..8]);
    base[8] = 0;
    for i in (0..8).rev() {
        if base[i] == 0x20 {
            base[i] = 0;
        } else {
            break;
        }
    }

    ext[..3].copy_from_slice(&name[8..11]);
    ext[3] = 0;
    for i in (0..3).rev() {
        if ext[i] == 0x20 {
            ext[i] = 0;
        } else {
            break;
        }
    }
}
