use core::ptr::addr_of_mut;

use core::arch::asm;

const PAGE_SIZE_4K: usize = 4096;
const PAGE_SIZE_2M: usize = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: usize = 512 * PAGE_SIZE_2M;

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct PageTable([u64; 512]); // same as std::array<uint64_t,512> (in memory-structure)

static mut PML4_TABLE: PageTable = PageTable([0; 512]);
static mut PDP_TABLE: PageTable = PageTable([0; 512]);
static mut PAGE_DIRECTORY: [PageTable; 64] = [PageTable([0; 512]); 64];

pub fn setup_identify_page_table() {
    unsafe {
        let mut pml4_table = (&mut *addr_of_mut!(PML4_TABLE)).0;
        let mut pdp_table = (&mut *addr_of_mut!(PDP_TABLE)).0;
        let page_directory = &mut *addr_of_mut!(PAGE_DIRECTORY);

        pml4_table[0] = (pdp_table.as_ptr() as u64) | 0x003;
        for i_pdpt in 0..page_directory.len() {
            pdp_table[i_pdpt] = (page_directory.as_ptr() as u64) | 0x003;
            for i_pd in 0..512 {
                page_directory[i_pdpt].0[i_pd] =
                    (i_pdpt * PAGE_SIZE_1G + i_pd * PAGE_SIZE_2M) as u64 | 0x083;
            }
        }

        let cr3_val = pml4_table.as_ptr() as u64;
        asm!(
            "mov cr3, {}",
            in(reg) cr3_val
        );
    }
}
