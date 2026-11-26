#![no_std]
#![no_main]

use core::arch::asm;
use core::ffi::{c_char, CStr};
use core::panic::PanicInfo;
use core::ptr::write_volatile;
use syscall::{exit, print, print_i64};

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn main(argc: i32, argv: *const *const u8) -> i32 {
    let cmd: &[u8] = if argc >= 2 {
        CStr::from_ptr(*argv.add(1) as *const c_char).to_bytes()
    } else {
        b"hlt"
    };

    match cmd {
        b"hlt" => {
            // Ring 3 からの特権命令実行 -> #GP (一般保護例外)
            asm!("hlt");
        }
        b"wr_kernel" => {
            // ユーザーモードからのスーパーバイザー領域への書き込み -> #PF (ページフォルト)
            write_volatile(0x100 as *mut i32, 42);
        }
        b"wr_app" => {
            // 未マップ領域への書き込み -> #PF (ページフォルト)
            write_volatile(0xffff_8000_ffff_0000 as *mut i32, 123);
        }
        b"zero" => {
            // Rust の `/` 演算子はゼロ除算時に CPU 例外ではなく panic_handler へ飛ぶため、
            // 直接 idiv 命令を実行して #DE (除算エラー例外) を発生させる
            let mut a: i32 = 100;
            let z: i32 = 0;
            asm!(
                "cdq",
                "idiv {z:e}",
                inout("eax") a,
                out("edx") _,
                z = in(reg) z,
            );
            print("100/0 = ");
            print_i64(a as i64);
            print("\n");
        }
        _ => {}
    }

    exit(0);
}
