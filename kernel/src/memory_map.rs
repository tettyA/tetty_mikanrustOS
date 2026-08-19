#[repr(C)]
pub struct MemoryMap {
    pub buffer_size: u64,
    pub buffer: *mut u8,
    pub map_size: u64,
    pub map_key: u64,
    pub descriptor_size: u64,
    pub descriptor_version: u32,
}
#[derive(Debug)]
#[repr(C)]
pub struct MemoryDescriptor {
    pub memory_type: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

#[derive(Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum MemoryType {
    kEfiReservedMemoryType = 0,
    kEfiLoaderCode = 1,
    kEfiLoaderData = 2,
    kEfiBootServicesCode = 3,
    kEfiBootServicesData = 4,
    kEfiRuntimeServicesCode = 5,
    kEfiRuntimeServicesData = 6,
    kEfiConventionalMemory = 7, // 空領域
    kEfiUnusableMemory = 8,
    kEfiACPIReclaimMemory = 9,
    kEfiACPIMemoryNVS = 10,
    kEfiMemoryMappedIO = 11,
    kEfiMemoryMappedIOPortSpace = 12,
    kEfiPalCode = 13,
    kEfiPersistentMemory = 14,
    kEfiMaxMemoryType = 15,
}

impl MemoryMap {}
/*
inline bool operator==(uint32_t lhs, MemoryType rhs) {
  return lhs == static_cast<uint32_t>(rhs);
}

inline bool operator==(MemoryType lhs, uint32_t rhs) {
  return rhs == lhs;
}
  */

impl MemoryDescriptor {
    pub fn memory_type(&self) -> MemoryType {
        if self.memory_type <= MemoryType::kEfiMaxMemoryType as u32 {
            unsafe { core::mem::transmute(self.memory_type) }
        } else {
            MemoryType::kEfiReservedMemoryType
        }
    }
}
impl MemoryType {
    pub fn is_available(&self) -> bool {
        match self {
            MemoryType::kEfiBootServicesCode
            | MemoryType::kEfiBootServicesData
            | MemoryType::kEfiConventionalMemory => true,
            _ => false,
        }
    }
}

const UEFI_PAGE_SIZE: usize = 4096;
