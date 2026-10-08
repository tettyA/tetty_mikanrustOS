use crate::global::Global;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct FrameID {
    pub id: usize,
}

type MapLineType = u64;
pub const BYTES_PER_FRAME: usize = 4 * 1024;
pub const UEFI_PAGE_SIZE: usize = 4096;
pub static MEMORY_MANAGER: Global<BitMapMemoryManager> = Global::new(BitMapMemoryManager::new());
pub struct BitMapMemoryManager {
    alloc_map: [MapLineType; Self::FRAME_COUNT / Self::BITS_PER_MAP_LINE],
    range_begin: FrameID,
    range_end: FrameID,
}
impl BitMapMemoryManager {
    // この目盛管理クラスで扱える最大の物理メモリ量(byte)
    const MAX_PHYSICAL_MEMORY_BYTES: usize = 128 * 1024 * 1024 * 1024;
    // MAX_PHYSICAL_MEMORY_BYTES までの物理法則メモリを扱うために必要なフレーム数
    const FRAME_COUNT: usize = Self::MAX_PHYSICAL_MEMORY_BYTES / BYTES_PER_FRAME;
    const BITMAP_LENGTH: usize = Self::FRAME_COUNT / Self::BITS_PER_MAP_LINE;
    // ビットマップ配列の一つの要素のビット数 == フレーム数
    const BITS_PER_MAP_LINE: usize = 8 * size_of::<MapLineType>();

    pub const fn new() -> Self {
        Self {
            alloc_map: [0; Self::BITMAP_LENGTH],
            range_begin: FrameID { id: 0 },
            range_end: FrameID {
                id: Self::FRAME_COUNT,
            },
        }
    }

    pub fn allocate(&mut self, num_frames: usize) -> Result<FrameID, ()> {
        let mut start_frame_id = self.range_begin.id;

        loop {
            let mut all_free = true;
            for i in 0..num_frames {
                if start_frame_id + i >= self.range_end.id {
                    return Err(());
                }
                if self.get_bit(FrameID {
                    id: start_frame_id + i,
                }) {
                    // start_frame_id + iにあるフレームには割り当て済み
                    all_free = false;
                    start_frame_id += i + 1;
                    break;
                }
            }
            if all_free {
                self.mark_allocated(FrameID { id: start_frame_id }, num_frames);
                return Ok(FrameID { id: start_frame_id });
            }
        }
    }
    pub fn free(&mut self, start_frame: FrameID, num_frames: usize) -> Result<(), ()> {
        for i in 0..num_frames {
            self.set_bit(
                FrameID {
                    id: start_frame.id + i,
                },
                false,
            );
        }
        return Ok(());
    }
    // 使用中の領域を設定する。
    pub fn mark_allocated(&mut self, start_frame: FrameID, num_frames: usize) {
        for i in 0..num_frames {
            self.set_bit(
                FrameID {
                    id: start_frame.id + i,
                },
                true,
            );
        }
    }

    pub fn set_memory_range(&mut self, range_begin: FrameID, range_end: FrameID) {
        self.range_begin = range_begin;
        self.range_end = range_end;
    }

    fn get_bit(&self, frame: FrameID) -> bool {
        let line_idx = frame.id / Self::BITS_PER_MAP_LINE;
        let bit_index = frame.id % Self::BITS_PER_MAP_LINE;

        self.alloc_map[line_idx] & (1u64 << bit_index) != 0
    }
    fn set_bit(&mut self, frame: FrameID, allocated: bool) {
        let line_idx = frame.id / Self::BITS_PER_MAP_LINE;
        let bit_index = frame.id % Self::BITS_PER_MAP_LINE;

        if allocated {
            self.alloc_map[line_idx] |= 1u64 << bit_index;
        } else {
            self.alloc_map[line_idx] &= !(1u64 << bit_index);
        }
    }
}
