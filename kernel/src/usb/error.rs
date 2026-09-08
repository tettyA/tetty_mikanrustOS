#![allow(non_camel_case_types)]

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    kSuccess, kFull, kEmpty, kNoEnoughMemory, kIndexOutOfRange,
    kHostControllerNotHalted, kInvalidSlotID, kPortNotConnected,
    kInvalidEndpointNumber, kTransferRingNotSet, kAlreadyAllocated,
    kNotImplemented, kInvalidDescriptor, kBufferTooSmall, kUnknownDevice,
    kNoCorrespondingSetupStage, kTransferFailed, kInvalidPhase,
    kUnknownXHCISpeedID, kNoWaiter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error { pub code: ErrorCode }
impl Error { pub const fn new(code: ErrorCode) -> Self { Self { code } } pub const fn ok() -> Self { Self::new(ErrorCode::kSuccess) } }
impl From<ErrorCode> for Error { fn from(code: ErrorCode) -> Self { Self::new(code) } }
pub type Result<T = ()> = core::result::Result<T, Error>;