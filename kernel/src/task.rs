use core::arch::asm;

use alloc::{boxed::Box, collections::VecDeque, vec::Vec};

use crate::{
    error::{KernelError, Result},
    global::Global,
    message::Message,
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

/// 実行可能なタスクが他に無い時に動く。level 0
extern "C" fn task_idle(_task_id: u64, _data: i64, _arg3: u64, _arg4: u64) {
    loop {
        x86_64::instructions::hlt();
    }
}

/// level: 0 = lowest, MAX_LEVEL = highest
pub const MAX_LEVEL: usize = 3;
pub const DEFAULT_LEVEL: usize = 1;

/// タスクのエントリ関数。引数は rdi, rsi, rdx, rcx (System V ABI)に渡される
pub type TaskFunc = extern "C" fn(u64, i64, u64, u64);

type SwitchPair = (*const TaskContext, *mut TaskContext);

/// タスク毎のスタックサイズ(バイト)
pub const DEFAULT_STACK_BYTES: usize = 8 * 1024; // format! とか使うから多めに

pub struct Task {
    id: u64,
    stack: Vec<u64>,
    context: TaskContext,
    msgs: VecDeque<Message>,
    level: usize,
    is_running: bool,
}

impl Task {
    fn new(id: u64) -> Self {
        Self {
            id,
            stack: Vec::new(),
            context: unsafe { core::mem::zeroed() },
            msgs: VecDeque::new(),
            level: DEFAULT_LEVEL,
            is_running: false,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
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

    fn context_ptr(&mut self) -> *mut TaskContext {
        &mut self.context
    }
}

pub struct TaskManager {
    tasks: Vec<Box<Task>>,
    latest_id: u64,
    running: [VecDeque<u64>; MAX_LEVEL + 1], // レベルごとの実行可能タスクのID。先頭にあるのが現在実行中,
    current_level: usize,
    level_changed: bool, // 次回タスクの切り替え時に実行中レベルを見直す必要があることを知らせる。trueにすると次SwitchTask()を実行中した時にcurrent_levelの見直しが行われる。
}

impl TaskManager {
    pub fn new() -> Self {
        let mut tm = Self {
            tasks: Vec::new(),
            latest_id: 0,
            running: core::array::from_fn(|_| VecDeque::new()),
            current_level: MAX_LEVEL,
            level_changed: false,
        };
        // 現在実行中のメインタスク。 contextは最初のSwitchContextで保存される。
        let level = tm.current_level;
        let task = tm.new_task();
        task.level = level;
        task.is_running = true;
        let id = task.id;
        tm.running[level].push_back(id);

        let idle = tm.new_task().init_context(task_idle, 0);
        idle.level = 0;
        idle.is_running = true;
        let idle_id = idle.id;
        tm.running[idle.level].push_back(idle_id);

        tm
    }

    /// 現在のタスクをrunningの末尾へと回す(寝る場合は外す)，次のタスクとの切り替え対象を返す。
    /// SwitchContext 自体は TASK_MANAGER.with の外で呼ぶこと。
    pub fn prepare_switch(&mut self, current_sleep: bool) -> SwitchPair {
        let level_queue = &mut self.running[self.current_level];
        let current_id = level_queue.pop_front().expect("no running task");
        if !current_sleep {
            level_queue.push_back(current_id);
        }
        if level_queue.is_empty() {
            self.level_changed = true;
            // このレベルで実行中可能なランキューが空になった
            // -> レベル更新
        }
        if self.level_changed {
            self.level_changed = false;
            // 実行可能なタスクがある最高レベルに変更
            for lv in (0..=MAX_LEVEL).rev() {
                if !self.running[lv].is_empty() {
                    self.current_level = lv;
                    break;
                }
            }
        }
        let next_id = *self.running[self.current_level]
            .front()
            .expect("no runnable task (idle task missing?)");

        let current = self.task_mut(current_id).unwrap().context_ptr();
        let next = self.task_mut(next_id).unwrap().context_ptr();
        (next as *const TaskContext, current)
    }

    pub fn new_task(&mut self) -> &mut Task {
        self.latest_id += 1;

        self.tasks.push(Box::new(Task::new(self.latest_id)));
        self.tasks.last_mut().unwrap()
    }
    fn task_mut(&mut self, id: u64) -> Option<&mut Task> {
        self.tasks.iter_mut().find(|t| t.id == id).map(|t| &mut **t)
    }

    /// 現在実行中のタスクを寝かせる場合だけ切り替え先を返す。
    pub fn sleep(&mut self, id: u64) -> Result<Option<SwitchPair>> {
        let task = self.task_mut(id).ok_or(KernelError::NoSuchTask)?;
        if !task.is_running {
            return Ok(None); // 既に寝ている。
        }

        task.is_running = false;
        let level = task.level;

        if self.running[self.current_level].front() == Some(&id) {
            // 現在実行中
            return Ok(Some(self.prepare_switch(true)));
        }
        self.running[level].retain(|&r| r != id); // C++でいうErase
        Ok(None)
    }

    /// `level` が `None` なら現在のレベルのまま。実行中のタスクに `Some` を渡すとレベルを変更する
    pub fn wakeup(&mut self, id: u64, level: Option<usize>) -> Result<()> {
        let task = self.task_mut(id).ok_or(KernelError::NoSuchTask)?;
        if task.is_running {
            self.change_level_running(id, level);
            return Ok(());
        }

        let level = level.unwrap_or(task.level);
        task.level = level;
        task.is_running = true;

        self.running[level].push_back(id);
        if level > self.current_level {
            self.level_changed = true;
        }

        Ok(())
    }

    /// 実行可能なタスクのレベルを変更する。`level` が None か現在と同じなら何もしない。
    fn change_level_running(&mut self, id: u64, level: Option<usize>) {
        let Some(level) = level else { return };
        let old_level = self.task_mut(id).unwrap().level;
        if level == old_level {
            return;
        }

        if self.running[self.current_level].front() != Some(&id) {
            // 他のタスクのレベルを変える。
            self.running[old_level].retain(|&r| r != id);
            self.running[level].push_back(id);
            self.task_mut(id).unwrap().level = level;
            if level > self.current_level {
                self.level_changed = true;
            }
            return;
        }

        // 自分自身のレベルを変更する。
        self.running[self.current_level].pop_front();
        self.running[level].push_front(id);
        self.task_mut(id).unwrap().level = level;
        let raised = level >= self.current_level;
        self.current_level = level;
        if !raised {
            self.level_changed = true;
        }
    }

    /// メッセージキューにメッセージを追加し，寝ていれば起こす。
    fn send_message(&mut self, id: u64, msg: Message) -> Result<()> {
        self.task_mut(id)
            .ok_or(KernelError::NoSuchTask)?
            .msgs
            .push_back(msg);
        // 起きているタスクを呼んでも無害
        self.wakeup(id, None)
    }
    /// メッセージキューから先頭のメッセージを1つ取り出す。
    fn receive_message(&mut self, id: u64) -> Option<Message> {
        self.task_mut(id)?.msgs.pop_front()
    }

    /// 現在実行中している(即ち，呼出元の)idを返す
    pub fn current_task_id(&self) -> u64 {
        *self.running[self.current_level].front().unwrap()
    }
}

pub static TASK_MANAGER: Global<Option<TaskManager>> = Global::new(None);

pub fn switch_task() {
    let pair = TASK_MANAGER.with(|tm| tm.as_mut().map(|tm| tm.prepare_switch(false)));
    if let Some((next, current)) = pair {
        unsafe { SwitchContext(next, current) };
    }
}

pub fn sleep(id: u64) -> Result<()> {
    x86_64::instructions::interrupts::without_interrupts(|| {
        let pair = TASK_MANAGER.with(|tm| tm.as_mut().unwrap().sleep(id))?;
        if let Some((next, current)) = pair {
            unsafe { SwitchContext(next, current) };
        }
        Ok(())
    })
}

/// 割込みハンドラからも呼ぶ。TASK_MANAGER未初期化ならErrを返す
pub fn send_message(id: u64, msg: Message) -> Result<()> {
    TASK_MANAGER.with(|tm| match tm {
        Some(tm) => tm.send_message(id, msg),
        None => Err(KernelError::NoSuchTask),
    })
}

pub fn receive_message(id: u64) -> Option<Message> {
    TASK_MANAGER.with(|tm| tm.as_mut().and_then(|tm| tm.receive_message(id)))
}

pub fn initialize_task() {
    TASK_MANAGER.with(|tm| *tm = Some(TaskManager::new()));

    TIMER_MANAGER.with(|tm| {
        let tm = tm.as_mut().unwrap();
        let timeout = tm.get_tick() + TASK_TIMER_PERIOD;
        tm.add_timer(&Timer::new(timeout, TASK_TIMER_VALUE));
    });
}
