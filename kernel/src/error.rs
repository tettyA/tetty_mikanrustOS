#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Error {
    kSuccess,
    kFull,
    kEmpty,
    kNoEnoughMemory,
    kIndexOutOfRange,
    kHostControllerNotHalted,
    kInvalidSlotID,
    kPortNotConnected,
    kInvalidEndpointNumber,
    kTransferFailed,
    kNoSuchEntry,
    kAlreadyExists,
    kNotImplemented,
    kInvalidFormat,
    kInvalidFile,
    kIsDirectory,
    kNoSuchTask,
}
