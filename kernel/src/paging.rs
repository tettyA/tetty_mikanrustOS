use crate::{
    error::Result,
    global::Global,
    memory_manager::{self, BYTES_PER_FRAME, FrameID, MEMORY_MANAGER},
};
use core::arch::asm;

const PAGE_SIZE_4K: usize = 4096;
const PAGE_SIZE_2M: usize = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: usize = 512 * PAGE_SIZE_2M;
/// 64ビット仮想アドレスを4階層ページテーブルのインデックスに分解する構造体
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinearAddress4Level(pub u64);
#[allow(dead_code)]
impl LinearAddress4Level {
    // 仮想アドレスの構成
    //  63           48 47     39 38     30 29     21 20     12 11            0
    //  +--------------+---------+---------+---------+---------+--------------+
    //  | 符号拡張(16)  | PML4(9) | PDPT(9) |  PD(9)  |  PT(9)  |  Offset(12)  |
    //  +--------------+---------+---------+---------+---------+--------------+

    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }
    pub const fn value(&self) -> u64 {
        self.0
    }
    pub const fn offset(&self) -> usize {
        (self.0 & 0xfff) as usize
    }
    pub const fn page(&self) -> usize {
        ((self.0 >> 12) & 0x1ff) as usize
    }
    pub const fn dir(&self) -> usize {
        ((self.0 >> 21) & 0x1ff) as usize
    }
    pub const fn pdp(&self) -> usize {
        ((self.0 >> 30) & 0x1ff) as usize
    }
    pub const fn pml4(&self) -> usize {
        ((self.0 >> 39) & 0x1ff) as usize
    }

    /// 階層レベル(4: PML4, 3: PDP, 2:PD 1:PT, 0: Offset)に対応する居んでっくすを取得
    pub fn part(&self, page_map_level: usize) -> usize {
        match page_map_level {
            1 => self.page(),
            2 => self.dir(),
            3 => self.pdp(),
            4 => self.pml4(),
            _ => self.offset(),
        }
    }

    /// 指定階層のインデックスを更新する。
    pub fn set_part(&mut self, page_map_level: usize, value: usize) {
        let val = (value & 0x1ff) as u64;
        match page_map_level {
            1 => self.0 = (self.0 & !(0x1ff << 12)) | (val << 12),
            2 => self.0 = (self.0 & !(0x1ff << 21)) | (val << 21),
            3 => self.0 = (self.0 & !(0x1ff << 30)) | (val << 30),
            4 => self.0 = (self.0 & !(0x1ff << 39)) | (val << 39),
            _ => self.0 = (self.0 & !0xfff) | ((value & 0xfff) as u64),
        }
    }
}

/// ページテーブルの1エントリ(64ビット)を表す型
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageMapEntry(pub u64);

#[allow(dead_code)]
impl PageMapEntry {
    pub const PRESENT: u64 = 1 << 0;
    pub const WRITABLE: u64 = 1 << 1;
    pub const USER: u64 = 1 << 2;
    pub const WRITE_THROUGH: u64 = 1 << 3;
    pub const CACHE_DISABLE: u64 = 1 << 4;
    pub const ACCESSED: u64 = 1 << 5;
    pub const DIRTY: u64 = 1 << 6;
    pub const HUGE_PAGE: u64 = 1 << 7;
    pub const GLOBAL: u64 = 1 << 8;

    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn data(&self) -> u64 {
        self.0
    }

    // 各フラグの判定
    pub const fn is_present(&self) -> bool {
        (self.0 & Self::PRESENT) != 0
    }
    pub const fn is_writable(&self) -> bool {
        (self.0 & Self::WRITABLE) != 0
    }
    pub const fn is_user(&self) -> bool {
        (self.0 & Self::USER) != 0
    }
    pub const fn is_write_through(&self) -> bool {
        (self.0 & Self::WRITE_THROUGH) != 0
    }
    pub const fn is_cache_disable(&self) -> bool {
        (self.0 & Self::CACHE_DISABLE) != 0
    }
    pub const fn is_accessed(&self) -> bool {
        (self.0 & Self::ACCESSED) != 0
    }
    pub const fn is_dirty(&self) -> bool {
        (self.0 & Self::DIRTY) != 0
    }
    pub const fn is_huge_page(&self) -> bool {
        (self.0 & Self::HUGE_PAGE) != 0
    }
    pub const fn is_global(&self) -> bool {
        (self.0 & Self::GLOBAL) != 0
    }

    // 各フラグ設定
    pub fn set_present(&mut self, value: bool) {
        self.set_bit(Self::PRESENT, value);
    }
    pub fn set_writable(&mut self, value: bool) {
        self.set_bit(Self::WRITABLE, value);
    }
    pub fn set_user(&mut self, value: bool) {
        self.set_bit(Self::USER, value);
    }
    pub fn set_write_through(&mut self, value: bool) {
        self.set_bit(Self::WRITE_THROUGH, value);
    }
    pub fn set_cache_disable(&mut self, value: bool) {
        self.set_bit(Self::CACHE_DISABLE, value);
    }
    pub fn set_accessed(&mut self, value: bool) {
        self.set_bit(Self::ACCESSED, value);
    }
    pub fn set_dirty(&mut self, value: bool) {
        self.set_bit(Self::DIRTY, value);
    }
    pub fn set_huge_page(&mut self, value: bool) {
        self.set_bit(Self::HUGE_PAGE, value);
    }
    pub fn set_global(&mut self, value: bool) {
        self.set_bit(Self::GLOBAL, value);
    }

    #[inline]
    fn set_bit(&mut self, bit: u64, value: bool) {
        if value {
            self.0 |= bit;
        } else {
            self.0 &= !bit;
        }
    }

    // アドレス，ポインタの操作
    /// 物理アドレスを取り出す
    pub const fn addr(&self) -> u64 {
        self.0 & 0x000f_ffff_ffff_f000
    }

    /// 次の階層のページテーブルへのポインタ
    pub fn pointer(&self) -> *mut PageMapEntry {
        self.addr() as *mut PageMapEntry
    }

    /// 次の階層のポインタを設定する
    pub fn set_pointer(&mut self, p: *const PageMapEntry) {
        let addr = p as u64;
        self.0 = (self.0 & !0x000f_ffff_ffff_f000) | (addr & 0x000f_ffff_ffff_f000);
    }
}

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct PageTable([u64; 512]); // same as std::array<uint64_t,512> (in memory-structure)

static PML4_TABLE: Global<PageTable> = Global::new(PageTable([0; 512]));
static PDP_TABLE: Global<PageTable> = Global::new(PageTable([0; 512]));
static PAGE_DIRECTORY: Global<[PageTable; 64]> = Global::new([PageTable([0; 512]); 64]);

pub fn setup_identify_page_table() {
    PML4_TABLE.with(|pml4_table| {
        PDP_TABLE.with(|pdp_table| {
            PAGE_DIRECTORY.with(|page_directory| unsafe {
                pml4_table.0[0] = (pdp_table.0.as_ptr() as u64) | 0x003;
                for i_pdpt in 0..page_directory.len() {
                    pdp_table.0[i_pdpt] = (page_directory[i_pdpt].0.as_ptr() as u64) | 0x003;
                    for i_pd in 0..512 {
                        page_directory[i_pdpt].0[i_pd] =
                            (i_pdpt * PAGE_SIZE_1G + i_pd * PAGE_SIZE_2M) as u64 | 0x083;
                    }
                }

                let cr3_val = pml4_table.0.as_ptr() as u64;
                asm!(
                    "mov cr3, {}",
                    in(reg) cr3_val
                );
            })
        })
    });
}

/// メモリマネージャから新しいページマップ（4KiB）を確保し、ゼロクリアして返す
fn new_page_map() -> Result<*mut PageMapEntry> {
    let frame = MEMORY_MANAGER.with(|mm| mm.allocate(1))?;
    let ptr = (frame.id * BYTES_PER_FRAME) as *mut PageMapEntry;
    unsafe {
        core::ptr::write_bytes(ptr as *mut u8, 0, 4096);
    }
    Ok(ptr)
}

/// エントリを確認して，まだ次の階層がなければ新しく確保して接続する。
fn set_new_page_map_if_not_present(entry: &mut PageMapEntry) -> Result<*mut PageMapEntry> {
    if entry.is_present() {
        return Ok(entry.pointer());
    }
    let child_map = new_page_map()?;
    entry.set_pointer(child_map);
    entry.set_present(true);
    Ok(child_map)
}

/// 指定レベルから下に向かって，再帰的にページエントリを設定する
fn setup_page_map(
    page_map: *mut PageMapEntry,
    page_map_level: usize,
    mut addr: LinearAddress4Level,
    mut num_4kpages: usize,
) -> Result<usize> {
    while num_4kpages > 0 {
        let entry_index = addr.part(page_map_level);
        let entry = unsafe { &mut *page_map.add(entry_index) };
        let child_map = set_new_page_map_if_not_present(entry)?;
        entry.set_writable(true);
        entry.set_user(true);

        if page_map_level == 1 {
            // レベル1に達成した = 1ページ分のマッピングが完了
            num_4kpages -= 1;
        } else {
            // 下のレベルに潜る
            let num_remain = setup_page_map(child_map, page_map_level - 1, addr, num_4kpages)?;
            num_4kpages = num_remain;
        }
        // テーブルの末尾(index:511)に達したら，このテーブル内ではこれ以上進めないので，抜ける
        if entry_index == 511 {
            break;
        }

        addr.set_part(page_map_level, entry_index + 1);
        for level in 1..page_map_level {
            addr.set_part(level, 0);
        }
    }

    Ok(num_4kpages)
}

/// 仮想アドレス `addr` から `num_4kpages` 分の4KiBページを現在のページテーブルに登録する
pub fn setup_page_maps(addr: LinearAddress4Level, num_4kpages: usize) -> Result<()> {
    let cr3: u64;
    unsafe {
        asm!("mov {}, cr3",out(reg) cr3);
    }
    let pml4_table = (cr3 & 0x000f_ffff_ffff_f000) as *mut PageMapEntry;
    setup_page_map(pml4_table, 4, addr, num_4kpages).map(|_| ())
}

/// 指定されたページング構造内のエントリを全て削除する。
fn clean_page_map(page_map: *mut PageMapEntry, page_map_level: usize) -> Result<()> {
    for i in 0..512 {
        let entry = unsafe { *page_map.add(i) };
        if !entry.is_present() {
            continue;
        }

        // レベル1よりも上の階層なら，子供たちを先に掃除する。
        if page_map_level > 1 {
            clean_page_map(entry.pointer(), page_map_level - 1)?;
        }

        let entry_addr = entry.addr() as usize;
        let map_frame = crate::memory_manager::FrameID {
            id: entry_addr / crate::memory_manager::BYTES_PER_FRAME,
        };
        crate::memory_manager::MEMORY_MANAGER.with(|mm| mm.free(map_frame, 1))?;
    }

    Ok(())
}

/// アプリ用のページング構造を破棄する。
pub fn clean_page_maps(addr: LinearAddress4Level) -> Result<()> {
    let cr3: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) cr3);
    }
    let pml4_table = (cr3 & 0x000f_ffff_ffff_f000) as *mut PageMapEntry;
    let pml4_entry = unsafe { &mut *pml4_table.add(addr.pml4()) };
    if !pml4_entry.is_present() {
        return Ok(());
    }
    let pdp_table = pml4_entry.pointer();
    // PML4 のdataを0にクリア(CPUから下位のツリーが見えなくなる)
    pml4_entry.0 = 0;

    clean_page_map(pdp_table, 3)?;
    let pdp_addr = pdp_table as usize;
    let pdp_frame = FrameID {
        id: pdp_addr / memory_manager::BYTES_PER_FRAME,
    };
    MEMORY_MANAGER.with(|mm| mm.free(pdp_frame, 1))
}
