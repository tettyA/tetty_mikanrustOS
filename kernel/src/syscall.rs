use core::arch::global_asm;

use crate::{
    graphics::{PixelDraw, Rectangle, Vector2D, to_color},
    layer::LAYER_MANAGER,
    message,
    task::{current_task_id, receive_message, sleep},
    timer::{self, TIMER_MANAGER},
    window::Window,
};

const IA32_EFER: u32 = 0xc000_0080;
const IA32_STAR: u32 = 0xc000_0081;
const IA32_LSTAR: u32 = 0xc000_0082;
const IA32_FMASK: u32 = 0xc000_0084;
fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    unsafe {
        core::arch::asm!("wrmsr" ,
    in("ecx") msr,
    in("eax") low,
    in("edx") high,
    options(nomem, nostack, preserves_flags))
    };
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallResult {
    pub value: u64,
    pub error: i32,
}

pub mod errno {
    pub const EPERM: i32 = 1;
    pub const E2BIG: i32 = 7;
    pub const EBADF: i32 = 9;
    pub const EFAULT: i32 = 14; // 不正なポインタ
    pub const EINVAL: i32 = 22;
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerArgs {
    pub timeout: u64,
    pub value: i32,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseMoveArgs {
    pub x: i32,
    pub y: i32,
    pub dx: i32,
    pub dy: i32,
    pub buttons: u8,
}
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseButtonArgs {
    pub x: i32,
    pub y: i32,
    pub press: i32, // 1: press, 0: release
    pub button: i32,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub union AppEventArgs {
    pub mouse_move: MouseMoveArgs,
    pub mouse_button: MouseButtonArgs,
    pub timer: TimerArgs,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AppEvent {
    pub event_type: u32,
    pub arg: AppEventArgs,
}

impl AppEvent {
    pub const fn empty() -> Self {
        Self {
            event_type: 0,
            arg: AppEventArgs {
                mouse_move: MouseMoveArgs {
                    x: 0,
                    y: 0,
                    dx: 0,
                    dy: 0,
                    buttons: 0,
                },
            },
        }
    }
}
pub mod app_event {
    pub const QUIT: u32 = 0;
    pub const MOUSE_MOVE: u32 = 1;
    pub const MOUSE_BUTTON: u32 = 2;
    pub const TIMER_TIMEOUT: u32 = 3;
}
// システムコール関数の型定義 (最大6引数、戻り値 SyscallResult)
pub type SyscallFn = extern "C" fn(u64, u64, u64, u64, u64, u64) -> SyscallResult;

/// システムコール 0x80000000: 文字列をコンソールに出力する
pub extern "C" fn sys_log_string(
    level: u64,
    message_ptr: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    if level > 3 {
        return SyscallResult {
            value: 0,
            error: errno::EPERM,
        };
    }
    let s_ptr = message_ptr as *const u8;
    if s_ptr.is_null() {
        return SyscallResult {
            value: 0,
            error: errno::EPERM,
        };
    }
    let mut len = 0;
    unsafe {
        while *s_ptr.add(len) != 0 {
            len += 1;
            if len > 1024 {
                return SyscallResult {
                    value: 0,
                    error: errno::E2BIG,
                };
            }
        }
        if let Ok(s) = core::str::from_utf8(core::slice::from_raw_parts(s_ptr, len)) {
            crate::printk!("{}", s);
        }
    }
    SyscallResult {
        value: len as u64,
        error: 0,
    }
}
/// システムコール 0x80000001: ターミナルに文字列を出力する (PutString)
pub extern "C" fn sys_put_string(
    fd: u64,
    buf_ptr: u64,
    len: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    if len > 1024 {
        return SyscallResult {
            value: 0,
            error: errno::E2BIG,
        };
    }
    if fd == 1 {
        let task_id = crate::task::current_task_id();
        let slice = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, len as usize) };
        crate::terminal::TERMINALS.with(|map| {
            if let Some(&term_ptr) = map.get(&task_id) {
                unsafe {
                    (*term_ptr).print_bytes(slice);
                }
            }
        });
        return SyscallResult {
            value: len,
            error: 0,
        };
    }
    SyscallResult {
        value: 0,
        error: errno::EBADF,
    }
}
/// システムコール 0x80000002 : アプリケーションを終了する
pub extern "C" fn sys_exit(
    exit_code: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    let os_stack_ptr = crate::task::current_task_os_stack_ptr();
    SyscallResult {
        value: os_stack_ptr,     // RAX に OSスタックポインタが入る
        error: exit_code as i32, // RDX に終了コードが入る
    }
}
/// システムコール 0x80000003 ウィンドウを開く
pub extern "C" fn sys_open_window(
    w: u64,
    h: u64,
    x: u64,
    y: u64,
    title_ptr: u64,
    _arg6: u64,
) -> SyscallResult {
    use crate::graphics::Vector2D;
    use crate::layer::LAYER_MANAGER;
    use crate::window::Window;

    let title = if title_ptr != 0 {
        let mut len = 0;
        unsafe {
            let p = title_ptr as *const u8;
            while *p.add(len) != 0 && len < 128 {
                len += 1;
            }
            core::str::from_utf8(core::slice::from_raw_parts(p, len)).unwrap_or("App Window")
        }
    } else {
        "App Window"
    };

    let layer_id = LAYER_MANAGER.with(|lm| {
        let lm = lm.as_mut().unwrap();
        let pixel_format = lm.pixel_format();
        let window = alloc::rc::Rc::new(core::cell::RefCell::new(Window::new_toplevel(
            w as usize,
            h as usize,
            pixel_format,
            title,
        )));

        let layer_id = lm.new_layer().set_window(window).set_draggable(true).id;

        lm.move_layer(
            layer_id,
            Vector2D {
                x: x as i32,
                y: y as i32,
            },
        );
        lm.activate(layer_id);
        layer_id
    });

    // レイヤーと現在のタスクを紐付ける
    let task_id = crate::task::current_task_id();

    crate::layer::register_layer_task(layer_id, task_id);

    SyscallResult {
        value: layer_id as u64,
        error: 0,
    }
}

fn do_win_func<F>(layer_id_flags: u64, f: F) -> SyscallResult
where
    F: FnOnce(&mut Window) -> Result<(), i32>,
{
    let layer_flags = (layer_id_flags >> 32) as u32;
    let layer_id = (layer_id_flags & 0xffff_ffff) as u32;

    let res = LAYER_MANAGER.with(|lm| {
        let Some(lm) = lm.as_mut() else {
            return Err(errno::EBADF);
        };
        let Some(layer) = lm.find_layer(layer_id) else {
            return Err(errno::EBADF);
        };
        let Some(window) = layer.window() else {
            return Err(errno::EBADF);
        };

        f(&mut window.borrow_mut())?;
        if (layer_flags & 1) == 0 {
            lm.draw_id(layer_id);
        }
        Ok(())
    });
    match res {
        Ok(()) => SyscallResult { value: 0, error: 0 },
        Err(err) => SyscallResult {
            value: 0,
            error: err,
        },
    }
}

/// システムコール 0x80000004: ウィンドウに文字列を描画 (WinWriteString)
pub extern "C" fn sys_win_write_stirng(
    layer_id_flags: u64,
    x: u64,
    y: u64,
    color: u64,
    s_ptr: u64,
    _arg6: u64,
) -> SyscallResult {
    let s = if s_ptr != 0 {
        let mut len = 0;
        unsafe {
            let p = s_ptr as *const u8;
            while *p.add(len) != 0 && len < 1024 {
                len += 1;
            }
            core::str::from_utf8(core::slice::from_raw_parts(p, len)).unwrap_or("")
        }
    } else {
        ""
    };

    let color = crate::graphics::to_color(color as u32);

    do_win_func(layer_id_flags, |window| {
        crate::font::write_string(&window.writer(), x as usize, y as usize, s, &color);
        Ok(())
    })
}

pub extern "C" fn sys_win_fill_rectrange(
    layer_id_flags: u64,
    x: u64,
    y: u64,
    w: u64,
    h: u64,
    color: u64,
) -> SyscallResult {
    let color = to_color(color as u32);
    let pos = Vector2D {
        x: x as usize,
        y: y as usize,
    };
    let size = Vector2D {
        x: w as usize,
        y: h as usize,
    };

    do_win_func(layer_id_flags, |window| {
        crate::graphics::fill_rectangle(&window.writer(), &pos, &size, &color);
        Ok(())
    })
}

pub extern "C" fn sys_get_current_tick(
    _arg1: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    let tick = TIMER_MANAGER.with(|tm| tm.as_ref().map_or(0, |tm| tm.get_tick()));
    SyscallResult {
        value: tick,
        error: crate::timer::TIMER_FREQ as i32,
    }
}
pub extern "C" fn sys_win_redraw(
    layer_id_flags: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    do_win_func(layer_id_flags, |_window| Ok(()))
}

pub extern "C" fn sys_win_draw_line(
    layer_id_flags: u64,
    x0: u64,
    y0: u64,
    x1: u64,
    y1: u64,
    color: u64,
) -> SyscallResult {
    let mut x0 = x0 as i32;
    let mut y0 = y0 as i32;
    let mut x1 = x1 as i32;
    let mut y1 = y1 as i32;
    let color = crate::graphics::to_color(color as u32);

    do_win_func(layer_id_flags, |window| {
        let sign = |x: i32| -> i32 {
            if x > 0 {
                1
            } else if x < 0 {
                -1
            } else {
                0
            }
        };
        let mut dx = x1 - x0 + sign(x1 - x0);
        let mut dy = y1 - y0 + sign(y1 - y0);

        if dx == 0 && dy == 0 {
            if x0 >= 0 && y0 >= 0 {
                window.writer().write(x0 as usize, y0 as usize, &color);
            }
            return Ok(());
        }

        // 傾きが緩やかな場合 (|dx| >= |dy|): x を 1 ずつ進めて y を計算
        if dx.abs() >= dy.abs() {
            if dx < 0 {
                core::mem::swap(&mut x0, &mut x1);
                core::mem::swap(&mut y0, &mut y1);
                dx = -dx;
                dy = -dy;
            }
            for x in x0..=x1 {
                let y = (dy as i64 * (x - x0) as i64) / dx as i64 + y0 as i64;
                if x >= 0 && y >= 0 {
                    window.writer().write(x as usize, y as usize, &color);
                }
            }
        } else {
            // 傾きが急な場合 (|dx| < |dy|): y を 1 ずつ進めて x を計算
            if dy < 0 {
                core::mem::swap(&mut x0, &mut x1);
                core::mem::swap(&mut y0, &mut y1);
                dx = -dx;
                dy = -dy;
            }
            for y in y0..=y1 {
                let x = (dx as i64 * (y - y0) as i64) / dy as i64 + x0 as i64;
                if x >= 0 && y >= 0 {
                    window.writer().write(x as usize, y as usize, &color);
                }
            }
        }
        Ok(())
    })
}

pub extern "C" fn sys_close_window(
    layer_id_flags: u64,
    _arg2: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    let layer_id = (layer_id_flags & 0xffff_ffff) as u32;

    let res = LAYER_MANAGER.with(|lm| {
        let Some(lm) = lm.as_mut() else {
            return Err(errno::EBADF);
        };
        let Some(layer) = lm.find_layer(layer_id) else {
            return Err(errno::EBADF);
        };

        let area = if let Some(window) = &layer.window() {
            let win = window.borrow();
            Rectangle {
                pos_lu: layer.pos,
                size: Vector2D {
                    x: win.width() as i32,
                    y: win.height() as i32,
                },
            }
        } else {
            return Err(errno::EBADF);
        };

        lm.activate(0);
        lm.remove_layer(layer_id);
        lm.draw_area(area);
        crate::layer::unregister_layer_task(layer_id);
        Ok(())
    });
    match res {
        Ok(()) => SyscallResult { value: 0, error: 0 },
        Err(err) => SyscallResult {
            value: 0,
            error: err,
        },
    }
}

/// システムコール 0x8000000a: イベント読み込み (ReadEvent)
pub extern "C" fn sys_read_event(
    events_ptr: u64,
    len: u64,
    _arg3: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    if events_ptr < 0x8000_0000_0000_0000 {
        return SyscallResult {
            value: 0,
            error: errno::EFAULT,
        };
    }
    let app_events = events_ptr as *mut AppEvent;
    let len = len as usize;
    let task_id = crate::task::current_task_id();
    let mut i = 0;

    while i < len {
        let msg = receive_message(task_id);
        if msg.is_none() && i == 0 {
            let _ = sleep(task_id);
            continue;
        }
        let Some(msg) = msg else {
            break;
        };

        match msg {
            crate::message::Message::KeyPush {
                keycode, modifier, ..
            } => {
                // keycode 20 は 'q' キー。modifier の bit0 (LControl) または bit4 (RControl)
                if keycode == 20 && (modifier & 0b0001_0001) != 0 {
                    unsafe {
                        (*app_events.add(i)).event_type = app_event::QUIT;
                    }
                    i += 1;
                }
            }
            message::Message::MouseMove {
                x,
                y,
                dx,
                dy,
                buttons,
            } => {
                unsafe {
                    let ev = &mut *app_events.add(i);
                    ev.event_type = app_event::MOUSE_MOVE;
                    ev.arg.mouse_move = MouseMoveArgs {
                        x,
                        y,
                        dx,
                        dy,
                        buttons,
                    };
                }
                i += 1;
            }

            message::Message::MouseButton {
                x,
                y,
                press,
                button,
            } => {
                unsafe {
                    let ev = &mut *app_events.add(i);
                    ev.event_type = app_event::MOUSE_BUTTON;
                    ev.arg.mouse_button = MouseButtonArgs {
                        x,
                        y,
                        press: if press { 1 } else { 0 },
                        button: button as i32,
                    }
                };
                i += 1;
            }
            message::Message::TimerTimeout { timeout, value } => {
                // アプリ用タイマは負の値で登録されている
                if value < 0 {
                    unsafe {
                        let ev = &mut *app_events.add(i);
                        ev.event_type = app_event::TIMER_TIMEOUT;
                        ev.arg.timer = TimerArgs { timeout, value };
                    }
                    i += 1;
                }
            }
            _ => {}
        }
    }

    SyscallResult {
        value: i as u64,
        error: 0,
    }
}
pub extern "C" fn sys_create_timer(
    mode: u64,
    timer_value: u64,
    timeout_ms: u64,
    _arg4: u64,
    _arg5: u64,
    _arg6: u64,
) -> SyscallResult {
    let timer_value = timer_value as i32;
    if timer_value <= 0 {
        return SyscallResult {
            value: 0,
            error: errno::EINVAL,
        };
    }

    let task_id = current_task_id();

    let mut timeout = timeout_ms * timer::TIMER_FREQ as u64 / 1000;

    if (mode & 1) != 0 {
        // 相対時間指定
        let current_tick = TIMER_MANAGER.with(|tm| tm.as_ref().map_or(0, |tm| tm.get_tick()));
        timeout += current_tick;
    }

    x86_64::instructions::interrupts::without_interrupts(|| {
        TIMER_MANAGER.with(|tm| {
            if let Some(tm) = tm.as_mut() {
                // アプリ用タイマは負数 (-timer_value) として登録する
                tm.add_timer(&crate::timer::Timer::new(timeout, -timer_value, task_id));
            }
        });
    });
    SyscallResult {
        value: timeout * 1000 / crate::timer::TIMER_FREQ as u64,
        error: 0,
    }
}
/// システムコールの関数ポインタテーブル (本家 syscall_table 相当)
#[unsafe(no_mangle)]
pub static SYSCALL_TABLE: [SyscallFn; 12] = [
    sys_log_string,
    sys_put_string,
    sys_exit,
    sys_open_window,
    sys_win_write_stirng,
    sys_win_fill_rectrange,
    sys_get_current_tick,
    sys_win_redraw,
    sys_win_draw_line,
    sys_close_window,
    sys_read_event,
    sys_create_timer,
];
#[unsafe(no_mangle)]
pub extern "C" fn GetCurrentTaskOSStackPointer() -> u64 {
    crate::task::current_task_os_stack_ptr()
}
global_asm!(
    r#"
.global CallApp
CallApp:  # int CallApp(int argc, char** argv, uint16_t ss, uint64_t rip, uint64_t rsp, uint64_t* os_stack_ptr);
    push rbx
    push rbp
    push r12
    push r13
    push r14
    push r15
    mov [r9], rsp  # OS 用のスタックポインタを保存 (第6引数 r9)
    push rdx       # SS (ユーザー空間データセグメント)
    push r8        # RSP (ユーザー空間スタックポインタ)
    add rdx, 8     # CS は SS の次 (USER_CS = USER_SS + 8)
    push rdx       # CS (ユーザー空間コードセグメント)
    push rcx       # RIP (エントリポイント)
    retfq          # ユーザーモード (Ring 3) へジャンプ
.global SyscallEntry
SyscallEntry:  # void SyscallEntry(void);
    push rbp
    push rcx   # ユーザー空間の元の RIP (syscall 命令が RCX に自動退避)
    push r11   # ユーザー空間の元の RFLAGS (syscall 命令が R11 に自動退避)
    push rax   # システムコール番号をスタックに保存
    mov rcx, r10        # syscall ABI では第4引数は R10 に入るため、C ABI の RCX に移動
    and eax, 0x7fffffff # システムコール番号の最上位ビットをマスク (0x80000000 -> 0)
    mov rbp, rsp
    # 全引数レジスタとシステムコール番号 (EAX) を一時退避
    push rax            # [rbp - 8]
    push rdi            # [rbp - 16]
    push rsi            # [rbp - 24]
    push rdx            # [rbp - 32]
    push rcx            # [rbp - 40]
    push r8             # [rbp - 48]
    push r9             # [rbp - 56]
    sub rsp, 8          # 16バイト・アライメント調整
    cli
    call GetCurrentTaskOSStackPointer
    sti
    # RAX に task.os_stack_ptr が返る
    # OS スタックに切り替え
    mov rsp, rax
    and rsp, -16
    # ユーザースタック (RBP) から全引数と EAX を復元
    mov r9,  [rbp - 56]
    mov r8,  [rbp - 48]
    mov rcx, [rbp - 40]
    mov rdx, [rbp - 32]
    mov rsi, [rbp - 24]
    mov rdi, [rbp - 16]
    mov eax, [rbp - 8]
    # システムコール関数を実行 (OS スタック上で動作)
    lea r11, [rip + SYSCALL_TABLE]
    call qword ptr [r11 + 8 * rax]
    # rax には value, rdx には error が格納されている
    # ユーザースタックに復帰
    mov rsp, rbp
    pop rsi             # 保存していたシステムコール番号を復帰
    cmp esi, 0x80000002 # Exit システムコール (0x80000002) か？
    je .Lexit           # Exit なら .Lexit へジャンプ
    pop r11    # 元の RFLAGS を復帰
    pop rcx    # 元の RIP を復帰
    pop rbp    # 元の RBP を復帰
    sysretq    # ユーザーモードへ復帰 (RIP <- RCX, RFLAGS <- R11)
.Lexit:
    # アプリ終了時のスタック復帰処理
    mov rsp, rax # rax は task.os_stack_ptr (CallApp 実行直前の OS スタックに復帰)
    mov eax, edx # edx は exit_code (第1戻り値として EAX に設定)
    pop r15
    pop r14
    pop r13
    pop r12
    pop rbp
    pop rbx
    ret          # CallApp の呼び出し元 (Terminal::ExecuteFile) へリターン
"#
);

unsafe extern "C" {
    pub fn SyscallEntry();
    pub fn CallApp(
        argc: i32,
        argv: *const *const u8,
        ss: u16,
        rip: u64,
        rsp: u64,
        os_stack_ptr: *mut u64,
    ) -> i32;
}
/// MSR を設定して syscall / sysret 命令を有効化する (本家 InitializeSyscall 相当)
pub fn init_syscall() {
    // 1. IA32_EFER: bit 0 (SCE) を 1 にして syscall 命令を有効化
    wrmsr(IA32_EFER, 0x0501);
    // 2. IA32_LSTAR: syscall 実行時のカーネルエントリポイントアドレスを設定
    wrmsr(IA32_LSTAR, SyscallEntry as *const () as usize as u64);
    // 3. IA32_STAR: syscall / sysret 実行時のセグメントセレクタを設定
    //    ・[47:32] = 8 (Kernel CS = 0x08, Kernel SS = 0x10)
    //    ・[63:48] = (16 | 3) (User SS = 0x1b, User CS = 0x23)
    wrmsr(IA32_STAR, (8u64 << 32) | (((16 | 3) as u64) << 48));
    // 4. IA32_FMASK: syscall 実行時の RFLAGS マスク (0 = マスクしない)
    wrmsr(IA32_FMASK, 0);
}
