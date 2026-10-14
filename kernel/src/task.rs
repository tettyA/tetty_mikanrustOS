use core::arch::asm;

use alloc::{boxed::Box, vec::Vec};

use crate::{
    global::Global,
    segment::{KERNEL_CS, KERNEL_SS},
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

/// タスクのエントリ関数。引数は rdi, rsi, rdx, rcx (System V ABI)に渡される
pub type TaskFunc = extern "C" fn(u64, i64, u64, u64);

/// タスク毎のスタックサイズ(バイト)
pub const DEFAULT_STACK_BYTES: usize = 8 * 1024; // format! とか使うから多めに

pub struct Task {
    id: u64,
    stack: Vec<u64>,
    context: TaskContext,
}

impl Task {
    fn new(id: u64) -> Self {
        Self {
            id,
            stack: Vec::new(),
            context: unsafe { core::mem::zeroed() },
        }
    }

    /// スタックを確保し，`f(id, data, 0, 0)`を実行するコンテクストを作る
    pub fn init_context(&mut self, f: TaskFunc, data: i64) -> &mut Self {
        let stack_size = DEFAULT_STACK_BYTES / core::mem::size_of::<u64>();
        self.stack = alloc::vec![0u64; stack_size];

        let stack_end = self.stack.as_ptr() as u64 + (stack_size * 8) as u64;

        self.context = unsafe { core::mem::zeroed() };
        self.context.cr3 = read_cr3();
        self.context.rflags = 0x202;
        self.context.cs = KERNEL_CS;
        self.context.ss = KERNEL_SS;
        self.context.rsp = (stack_end & !0xf) - 8;

        self.context.rip = f as usize as u64;
        self.context.rdi = self.id;
        self.context.rsi = data as u64;

        // MXCSR の全ての例外をマスクする
        self.context.fxsave_area[24..28].copy_from_slice(&0x1f80u32.to_le_bytes());

        self
    }

    pub fn set_extra_args(&mut self, arg3: u64, arg4: u64) -> &mut Self {
        self.context.rdx = arg3;
        self.context.rcx = arg4;
        self
    }

    fn context_ptr(&mut self) -> *mut TaskContext {
        &mut self.context
    }
}

pub struct TaskManager {
    tasks: Vec<Box<Task>>,
    latest_id: u64,
    current_task_index: usize,
}

impl TaskManager {
    pub fn new() -> Self {
        let mut tm = Self {
            tasks: Vec::new(),
            latest_id: 0,
            current_task_index: 0,
        };
        // 現在実行中のメインタスク。 contextは最初のSwitchContextで保存される。
        tm.new_task();
        tm
    }

    pub fn new_task(&mut self) -> &mut Task {
        self.latest_id += 1;

        self.tasks.push(Box::new(Task::new(self.latest_id)));
        self.tasks.last_mut().unwrap()
    }

    /// 次のタスクを決めて current_task_index を進め、(next, current) のコンテキストを返す。
    /// SwitchContext 自体は TASK_MANAGER.with の外で呼ぶこと。
    fn prepare_switch(&mut self) -> (*const TaskContext, *mut TaskContext) {
        let mut next_task_index = self.current_task_index + 1;
        if next_task_index >= self.tasks.len() {
            next_task_index = 0;
        }

        let current = self.tasks[self.current_task_index].context_ptr();
        let next = self.tasks[next_task_index].context_ptr();
        self.current_task_index = next_task_index;

        (next as *const TaskContext, current)
    }
}

pub static TASK_MANAGER: Global<Option<TaskManager>> = Global::new(None);

pub fn switch_task() {
    let pointers = TASK_MANAGER.with(|tm| tm.as_mut().map(|tm| tm.prepare_switch()));
    if let Some((next, current)) = pointers {
        unsafe { SwitchContext(next, current) };
    }
}

pub fn initialize_task() {
    TASK_MANAGER.with(|tm| *tm = Some(TaskManager::new()));

    TIMER_MANAGER.with(|tm| {
        let tm = tm.as_mut().unwrap();
        let timeout = tm.get_tick() + TASK_TIMER_PERIOD;
        tm.add_timer(&Timer::new(timeout, TASK_TIMER_VALUE));
    });
}
