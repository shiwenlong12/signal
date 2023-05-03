//! 信号的管理和处理模块
//!
//! 信号模块的实际实现见 `signal_impl` 子模块
//!
//!

#![no_std]

extern crate alloc;
use alloc::boxed::Box;
#[cfg(feature = "user")]
/// 线程上下文。
#[derive(Clone)]
#[repr(C)]
pub struct LocalContext {
    sctx: usize,
    x: [usize; 31],
    sepc: usize,
    /// 是否以特权态切换。
    pub supervisor: bool,
    /// 线程中断是否开启。
    pub interrupt: bool,
}

#[cfg(feature = "user")]
impl LocalContext {
    /// 创建空白上下文。
    #[inline]
    pub const fn empty() -> Self {
        Self {
            sctx: 0,
            x: [0; 31],
            supervisor: false,
            interrupt: false,
            sepc: 0,
        }
    }

    /// 初始化指定入口的用户上下文。
    ///
    /// 切换到用户态时会打开内核中断。
    #[inline]
    pub const fn user(pc: usize) -> Self {
        Self {
            sctx: 0,
            x: [0; 31],
            supervisor: false,
            interrupt: true,
            sepc: pc,
        }
    }

    /// 初始化指定入口的内核上下文。
    #[inline]
    pub const fn thread(pc: usize, interrupt: bool) -> Self {
        Self {
            sctx: 0,
            x: [0; 31],
            supervisor: true,
            interrupt,
            sepc: pc,
        }
    }

    /// 读取用户通用寄存器。
    #[inline]
    pub fn x(&self, n: usize) -> usize {
        self.x[n - 1]
    }

    /// 修改用户通用寄存器。
    #[inline]
    pub fn x_mut(&mut self, n: usize) -> &mut usize {
        &mut self.x[n - 1]
    }

    /// 读取用户参数寄存器。
    #[inline]
    pub fn a(&self, n: usize) -> usize {
        self.x(n + 10)
    }

    /// 修改用户参数寄存器。
    #[inline]
    pub fn a_mut(&mut self, n: usize) -> &mut usize {
        self.x_mut(n + 10)
    }

    /// 读取用户栈指针。
    #[inline]
    pub fn ra(&self) -> usize {
        self.x(1)
    }

    /// 读取用户栈指针。
    #[inline]
    pub fn sp(&self) -> usize {
        self.x(2)
    }

    /// 修改用户栈指针。
    #[inline]
    pub fn sp_mut(&mut self) -> &mut usize {
        self.x_mut(2)
    }

    /// 当前上下文的 pc。
    #[inline]
    pub fn pc(&self) -> usize {
        self.sepc
    }

    /// 修改上下文的 pc。
    #[inline]
    pub fn pc_mut(&mut self) -> &mut usize {
        &mut self.sepc
    }

    /// 将 pc 移至下一条指令。
    ///
    /// # Notice
    ///
    /// 假设这一条指令不是压缩版本。
    #[inline]
    pub fn move_next(&mut self) {
        self.sepc = self.sepc.wrapping_add(4);
    }

}

#[cfg(feature = "kernel")]
use kernel_context::LocalContext;
pub use signal_defs::{SignalAction, SignalNo, MAX_SIG};

mod signal_result;
pub use signal_result::SignalResult;

/// 一个信号模块需要对外暴露的接口
pub trait Signal: Send + Sync {
    /// 当 fork 一个任务时(在通常的`linux syscall`中，fork是某种参数形式的sys_clone)，
    /// 需要**继承原任务的信号处理函数和掩码**。
    /// 此时 `task` 模块会调用此函数，根据原任务的信号模块生成新任务的信号模块
    fn from_fork(&mut self) -> Box<dyn Signal>;

    /// `sys_exec`会使用。** `sys_exec` 不会继承信号处理函数和掩码**
    fn clear(&mut self);

    /// 添加一个信号
    fn add_signal(&mut self, signal: SignalNo);

    /// 是否当前正在处理信号
    fn is_handling_signal(&self) -> bool;

    /// 设置一个信号处理函数，返回设置是否成功。`sys_sigaction` 会使用。
    /// （**不成功说明设置是无效的，需要在 sig_action 中返回EINVAL**）
    fn set_action(&mut self, signum: SignalNo, action: &SignalAction) -> bool;

    /// 获取一个信号处理函数的值，返回设置是否成功。`sys_sigaction` 会使用
    ///（**不成功说明设置是无效的，需要在 sig_action 中返回EINVAL**）
    fn get_action_ref(&self, signum: SignalNo) -> Option<SignalAction>;

    /// 设置信号掩码，并获取旧的信号掩码，`sys_procmask` 会使用
    fn update_mask(&mut self, mask: usize) -> usize;

    /// 进程执行结果，可能是直接返回用户程序或存栈或暂停或退出
    fn handle_signals(&mut self, current_context: &mut LocalContext) -> SignalResult;

    /// 从信号处理函数中退出，返回值表示是否成功。`sys_sigreturn` 会使用
    fn sig_return(&mut self, current_context: &mut LocalContext) -> bool;
}
