//! カーネル共通のエラー型。
//!
//! 本家 MikanOS の `Error` クラス (`error.hpp`) に相当する。
//! 本家は 25 個の列挙子を最初から持つが、ここでは実際に使うものだけを定義し、
//! 必要になった時に足す。
use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelError {
    /// 空きフレームが無い (本家の `kNoEnoughMemory`)
    NoEnoughMemory,
    /// ELF など、ファイルの構造が想定と違う (本家の `kInvalidFormat`)
    InvalidFormat,
    /// 指定された ID のタスクが存在しない (本家の `kNoSuchTask`)
    NoSuchTask,
}

/// カーネル内の処理結果。エラー側は常に [`KernelError`]。
pub type Result<T> = core::result::Result<T, KernelError>;

impl fmt::Display for KernelError {
    /// 本家の `Error::Name()` 相当。画面表示用に列挙子名をそのまま出す。
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
