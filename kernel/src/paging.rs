use crate::global::Global;
use core::arch::asm;

const PAGE_SIZE_4K: usize = 4096;
const PAGE_SIZE_2M: usize = 512 * PAGE_SIZE_4K;
const PAGE_SIZE_1G: usize = 512 * PAGE_SIZE_2M;

#[repr(align(4096))]
#[derive(Clone, Copy)]
struct PageTable([u64; 512]); // same as std::array<uint64_t,512> (in memory-structure)

static PML4_TABLE: Global<PageTable> = Global::new(PageTable([0; 512]));
static PDP_TABLE: Global<PageTable> = Global::new(PageTable([0; 512]));
static PAGE_DIRECTORY: Global<[PageTable; 64]> = Global::new([PageTable([0; 512]); 64]);

pub fn setup_identify_page_table() {
    PML4_TABLE.with(|pml4_table| PDP_TABLE.with(|pdp_table| PAGE_DIRECTORY.with(|page_directory| unsafe {

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
    })));
}
