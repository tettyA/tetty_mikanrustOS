use core::arch::asm;

use crate::{
    global::Global,
    timer::{TASK_TIMER_PERIOD, TASK_TIMER_VALUE, TIMER_MANAGER, Timer},
};

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct TaskContext {
    // offset 0x00
    pub cr3: u64,
    pub rip: u64,
    pub rflags: u64,
    pub reserved1: u64,

    // offset 0x20
    pub cs: u64,
    pub ss: u64,
    pub fs: u64,
    pub gs: u64,

    // offset 0x40
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rsp: u64,
    pub rbp: u64,

    // offset 0x80
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    // offset 0xc0
    pub fxsave_area: [u8; 512],
}

core::arch::global_asm!(
    r#"
.global SwitchContext
SwitchContext:  # void SwitchContext(void* next_ctx, void* current_ctx);
    mov [rsi + 0x40], rax
    mov [rsi + 0x48], rbx
    mov [rsi + 0x50], rcx
    mov [rsi + 0x58], rdx
    mov [rsi + 0x60], rdi
    mov [rsi + 0x68], rsi
    lea rax, [rsp + 8]
    mov [rsi + 0x70], rax   # RSP
    mov [rsi + 0x78], rbp
    mov [rsi + 0x80], r8
    mov [rsi + 0x88], r9
    mov [rsi + 0x90], r10
    mov [rsi + 0x98], r11
    mov [rsi + 0xa0], r12
    mov [rsi + 0xa8], r13
    mov [rsi + 0xb0], r14
    mov [rsi + 0xb8], r15
    mov rax, cr3
    mov [rsi + 0x00], rax   # CR3
    mov rax, [rsp]
    mov [rsi + 0x08], rax   # RIP
    pushfq
    pop qword ptr [rsi + 0x10]  # RFLAGS
    mov ax, cs
    mov [rsi + 0x20], rax
    mov bx, ss
    mov [rsi + 0x28], rbx
    mov cx, fs
    mov [rsi + 0x30], rcx
    mov dx, gs
    mov [rsi + 0x38], rdx
    fxsave [rsi + 0xc0]
    # iret 用のスタックフレーム
    push qword ptr [rdi + 0x28]  # SS
    push qword ptr [rdi + 0x70]  # RSP
    push qword ptr [rdi + 0x10]  # RFLAGS
    push qword ptr [rdi + 0x20]  # CS
    push qword ptr [rdi + 0x08]  # RIP
    # コンテキストの復帰
    fxrstor [rdi + 0xc0]
    mov rax, [rdi + 0x00]
    mov cr3, rax
    mov rax, [rdi + 0x30]
    mov fs, ax
    mov rax, [rdi + 0x38]
    mov gs, ax
    mov rax, [rdi + 0x40]
    mov rbx, [rdi + 0x48]
    mov rcx, [rdi + 0x50]
    mov rdx, [rdi + 0x58]
    mov rsi, [rdi + 0x68]
    mov rbp, [rdi + 0x78]
    mov r8,  [rdi + 0x80]
    mov r9,  [rdi + 0x88]
    mov r10, [rdi + 0x90]
    mov r11, [rdi + 0x98]
    mov r12, [rdi + 0xa0]
    mov r13, [rdi + 0xa8]
    mov r14, [rdi + 0xb0]
    mov r15, [rdi + 0xb8]
    mov rdi, [rdi + 0x60]
    iretq
"#
);
unsafe extern "C" {
    pub fn SwitchContext(next_ctx: *const TaskContext, current_ctx: *mut TaskContext);
}
pub fn read_cr3() -> u64 {
    let value: u64;
    unsafe {
        asm!("mov {}, cr3", out(reg) value, options(nomem,nostack,preserves_flags));
    }
    value
}

pub static TASK_A_CTX: Global<TaskContext> = Global::new(unsafe { core::mem::zeroed() });
pub static TASK_B_CTX: Global<TaskContext> = Global::new(unsafe { core::mem::zeroed() });
pub static CURRENT_TASK: Global<*mut TaskContext> = Global::new(core::ptr::null_mut());

pub fn switch_task() {
    let task_a = TASK_A_CTX.with(|c| c as *mut TaskContext);
    let task_b = TASK_B_CTX.with(|c| c as *mut TaskContext);

    let (next, old) = CURRENT_TASK.with(|cur| {
        let old = *cur;
        if old == task_a {
            *cur = task_b;
        } else {
            *cur = task_a;
        }
        (*cur, old)
    });

    unsafe {
        SwitchContext(next, old);
    }
}

pub fn initialize_task() {
    let task_a = TASK_A_CTX.with(|c| c as *mut TaskContext);

    // 最初はtask_a
    CURRENT_TASK.with(|current| *current = task_a);
    // Global::with は割込みを禁止している(実装を見よ)のでcliとstiは不要
    TIMER_MANAGER.with(|tm| {
        let tm = tm.as_mut().unwrap();
        let timeout = tm.get_tick() + TASK_TIMER_PERIOD;
        tm.add_timer(&Timer::new(timeout, TASK_TIMER_VALUE));
    })
}
