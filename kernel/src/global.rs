//! `Global<T>` : `static mut` の代わりに使う、単一コア向けのグローバル変数ラッパー。
//!
//! # 使い方
//! ```ignore
//! static COUNTER: Global<u32> = Global::new(0);
//! COUNTER.with(|c| *c += 1);          // 割り込みを止めて &mut T を借りる
//! ```
//! - `with` の間は割り込みが禁止される (終了時に元の状態へ戻す) ので、割り込みハンドラとの競合が起きない。
//! - 同じ `Global` を `with` の中でもう一度 `with` する(再入)とパニックする。aliasing した `&mut` を作らないための検査。
//! - `with` の中で時間のかかる処理 (描画など) をすると、その間 xHCI 割り込みが遅れる。
//!
//! 前提: シングルコアで動くこと (マルチコア化するなら spin ロックに置き換える)。
use core::cell::{Cell, UnsafeCell};
use x86_64::instructions::interrupts::without_interrupts;

pub struct Global<T> {
    value: UnsafeCell<T>,
    busy: Cell<bool>,
}

// シングルコア + 割り込み禁止 + 再入検査により、同時に &mut T が複数存在することは無い。
unsafe impl<T> Sync for Global<T> {}

impl<T> Global<T> {
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(value),
            busy: Cell::new(false),
        }
    }

    /// 割り込みを禁止して `&mut T` を `f` に渡す。再入時はパニック。
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        without_interrupts(|| {
            assert!(!self.busy.get(), "Global: re-entrant access");
            self.busy.set(true);
            let r = f(unsafe { &mut *self.value.get() });
            self.busy.set(false);
            r
        })
    }

    /// GDT / IDT のように、CPU が以後ずっと参照するため `'static` な参照が要る初期化専用。
    /// 初期化時に 1 回だけ呼ぶこと。
    pub fn init_static<R>(&'static self, f: impl FnOnce(&'static mut T) -> R) -> R {
        without_interrupts(|| {
            assert!(!self.busy.get(), "Global: re-entrant access");
            f(unsafe { &mut *self.value.get() })
        })
    }
}
