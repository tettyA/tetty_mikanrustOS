use crate::memory_manager::{BYTES_PER_FRAME, MEMORY_MANAGER};
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();
// このメモリマネージャには64*512フレームだけメモリを割り当ててもらう。
const HEAP_FRAMES: usize = 64 * 512;
pub fn init_heap() {
    {
        match MEMORY_MANAGER.with(|manager| manager.allocate(HEAP_FRAMES)) {
            Ok(start_frame) => {
                let heap_start = start_frame.id * BYTES_PER_FRAME;
                let heap_size = HEAP_FRAMES * BYTES_PER_FRAME;
                unsafe { ALLOCATOR.lock().init(heap_start as *mut u8, heap_size) };
            }
            Err(_) => {
                panic!("Failed to allocate pages\n");
            }
        }
    }
}
