use crate::memory_manager::{BYTES_PER_FRAME, MEMORY_MANAGER};
use core::alloc::{GlobalAlloc, Layout};
use linked_list_allocator::LockedHeap;
use x86_64::instructions::interrupts::without_interrupts;

// 確保，解放の間は割込み禁止
struct InterruptSafeHeap(LockedHeap);
unsafe impl GlobalAlloc for InterruptSafeHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        without_interrupts(|| unsafe { self.0.alloc(layout) })
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        without_interrupts(|| unsafe {
            self.0.dealloc(ptr, layout);
        })
    }
}

#[global_allocator]
static ALLOCATOR: InterruptSafeHeap = InterruptSafeHeap(LockedHeap::empty());
// このメモリマネージャには64*512フレームだけメモリを割り当ててもらう。
const HEAP_FRAMES: usize = 64 * 512;
pub fn init_heap() {
    {
        match MEMORY_MANAGER.with(|manager| manager.allocate(HEAP_FRAMES)) {
            Ok(start_frame) => {
                let heap_start = start_frame.id * BYTES_PER_FRAME;
                let heap_size = HEAP_FRAMES * BYTES_PER_FRAME;
                unsafe { ALLOCATOR.0.lock().init(heap_start as *mut u8, heap_size) };
            }
            Err(_) => {
                panic!("Failed to allocate pages\n");
            }
        }
    }
}
