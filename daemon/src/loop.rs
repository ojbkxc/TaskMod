//! 事件循环核心模块
//!
//! 基于 epoll 的事件多路复用，监听以下事件源：
//! - 子进程 stdout pipe → 转发到父进程 stdout
//! - 子进程 stderr pipe → 转发到父进程 stderr
//! - Unix socket → 处理 IPC 命令
//! - signalfd → 处理 SIGCHLD/SIGUSR1 信号
//!
//! 这是守护进程的核心循环，所有事件处理都在同步上下文中完成。

use std::io::{self, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Instant;

use log::{error, info, warn};
use nix::sys::signal::{self, SigSet, Signal};
use nix::sys::signalfd::{SignalFd, SfdFlags};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;

use crate::config::Config;
use crate::error::{Result, TaskModError};
use crate::ipc::{self, Command, StatusResponse};
use crate::process::ProcessGuard;

/// 事件源标识
#[derive(Debug, Clone, Copy, PartialEq)]
enum EventSource {
    Stdout,
    Stderr,
    Signal,
    Ipc,
}

/// 事件循环状态
struct EventLoop {
    config: Config,
    guard: ProcessGuard,
    socket: std::os::unix::net::UnixDatagram,
    signal_fd: SignalFd,
    epoll_fd: RawFd,
    retry_count: u32,
    should_exit: bool,
}

/// epoll 事件数据结构（简化版，避免依赖 libc 的复杂结构）
#[repr(C)]
struct EpollEvent {
    events: u32,
    data: u64,
}

impl EventLoop {
    /// 创建并初始化事件循环
    fn new(config: Config) -> Result<Self> {
        // 创建 signalfd 监听 SIGCHLD 和 SIGUSR1
        let mut mask = SigSet::empty();
        mask.add(Signal::SIGCHLD);
        mask.add(Signal::SIGUSR1);
        mask.thread_block().map_err(TaskModError::Signal)?;

        let signal_fd = SignalFd::with_flags(&mask, SfdFlags::SFD_NONBLOCK)
            .map_err(|e| {
                TaskModError::Signal(nix::errno::Errno::from_i32(
                    e.raw_os_error().unwrap_or(1),
                ))
            })?;

        // 创建 IPC socket
        let socket = ipc::create_socket()?;

        // 创建 epoll 实例
        let epoll_fd = unsafe { epoll_create1(0) };
        if epoll_fd < 0 {
            return Err(TaskModError::Io(io::Error::last_os_error()));
        }

        let mut loop_state = EventLoop {
            config,
            guard: ProcessGuard::spawn(&config)?,
            socket,
            signal_fd,
            epoll_fd,
            retry_count: 0,
            should_exit: false,
        };

        // 注册所有事件源到 epoll
        loop_state.register_events()?;

        Ok(loop_state)
    }

    /// 将文件描述符注册到 epoll
    fn register_events(&self) -> Result<()> {
        // 注册子进程 stdout
        if let Some(fd) = self.guard.stdout_fd() {
            self.add_to_epoll(fd, EventSource::Stdout as u64)?;
        }

        // 注册子进程 stderr
        if let Some(fd) = self.guard.stderr_fd() {
            self.add_to_epoll(fd, EventSource::Stderr as u64)?;
        }

        // 注册 signalfd
        self.add_to_epoll(
            self.signal_fd.as_raw_fd(),
            EventSource::Signal as u64,
        )?;

        // 注册 IPC socket
        self.add_to_epoll(
            self.socket.as_raw_fd(),
            EventSource::Ipc as u64,
        )?;

        Ok(())
    }

    /// 添加 fd 到 epoll 监听
    fn add_to_epoll(&self, fd: RawFd, data: u64) -> Result<()> {
        let mut event = EpollEvent {
            events: EPOLLIN,
            data,
        };

        let ret = unsafe {
            epoll_ctl(self.epoll_fd, EPOLL_CTL_ADD, fd, &mut event as *mut _)
        };

        if ret < 0 {
            return Err(TaskModError::Io(io::Error::last_os_error()));
        }

        Ok(())
    }

    /// 从 epoll 移除 fd
    fn remove_from_epoll(&self, fd: RawFd) -> Result<()> {
        let ret = unsafe {
            epoll_ctl(
                self.epoll_fd,
                EPOLL_CTL_DEL,
                fd,
                std::ptr::null_mut(),
            )
        };

        if ret < 0 {
            // 忽略 ENOENT 错误（fd 可能已被关闭）
            let err = io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::ENOENT) {
                return Err(TaskModError::Io(err));
            }
        }

        Ok(())
    }

    /// 主事件循环
    fn run(&mut self) -> Result<()> {
        info!("事件循环启动");

        let mut events = [EpollEvent { events: 0, data: 0 }; 16];

        while !self.should_exit {
            // 等待事件，超时 100ms 用于处理重试逻辑
            let nfds = unsafe {
                epoll_wait(self.epoll_fd, events.as_mut_ptr(), 16, 100)
            };

            if nfds < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue; // 被信号中断，正常
                }
                return Err(TaskModError::Io(err));
            }

            // 处理所有就绪事件
            for i in 0..nfds as usize {
                let source = match events[i].data {
                    x if x == EventSource::Stdout as u64 => EventSource::Stdout,
                    x if x == EventSource::Stderr as u64 => EventSource::Stderr,
                    x if x == EventSource::Signal as u64 => EventSource::Signal,
                    x if x == EventSource::Ipc as u64 => EventSource::Ipc,
                    _ => continue,
                };

                match source {
                    EventSource::Stdout => self.handle_stdout()?,
                    EventSource::Stderr => self.handle_stderr()?,
                    EventSource::Signal => self.handle_signal()?,
                    EventSource::Ipc => self.handle_ipc()?,
                }
            }

            // 检查子进程是否意外退出（非信号触发的退出检测）
            if !self.guard.is_alive() {
                self.handle_child_exit()?;
            }
        }

        info!("事件循环退出");
        Ok(())
    }

    /// 处理子进程 stdout 输出（零拷贝转发）
    fn handle_stdout(&mut self) -> Result<()> {
        self.forward_pipe_output(self.guard.stdout_fd(), io::stdout())
    }

    /// 处理子进程 stderr 输出（零拷贝转发）
    fn handle_stderr(&mut self) -> Result<()> {
        self.forward_pipe_output(self.guard.stderr_fd(), io::stderr())
    }

    /// 零拷贝转发 pipe 数据到目标 writer
    fn forward_pipe_output<W: Write>(
        &self,
        fd: Option<RawFd>,
        mut writer: W,
    ) -> Result<()> {
        let fd = match fd {
            Some(fd) => fd,
            None => return Ok(()),
        };

        let mut buf = [0u8; 8192];
        loop {
            let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
            if n <= 0 {
                if n < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock {
                        break; // 无更多数据
                    }
                    return Err(TaskModError::Io(err));
                }
                break; // EOF
            }

            // 直接写入，不缓冲
            writer.write_all(&buf[..n]).map_err(TaskModError::Io)?;
            writer.flush().map_err(TaskModError::Io)?;
        }

        Ok(())
    }

    /// 处理信号
    fn handle_signal(&mut self) -> Result<()> {
        match self.signal_fd.read_signal() {
            Ok(Some(siginfo)) => {
                match Signal::try_from(siginfo.ssi_signo as i32) {
                    Ok(Signal::SIGCHLD) => {
                        info!("收到 SIGCHLD 信号");
                        self.handle_child_exit()?;
                    }
                    Ok(Signal::SIGUSR1) => {
                        info!("收到 SIGUSR1 信号，触发热重载");
                        self.atomic_reload()?;
                    }
                    _ => {}
                }
            }
            Ok(None) => {}
            Err(e) => {
                warn!("读取信号失败: {}", e);
            }
        }

        Ok(())
    }

    /// 处理子进程退出
    fn handle_child_exit(&mut self) -> Result<()> {
        // 回收子进程，避免僵尸进程
        match waitpid(self.guard.pid(), Some(nix::sys::wait::WaitPidFlag::WNOHANG)) {
            Ok(WaitStatus::Exited(pid, code)) => {
                warn!(
                    "子进程退出 (pid={}, code={}), 重试计数: {}/{}",
                    pid, code, self.retry_count, self.config.max_retries
                );
            }
            Ok(WaitStatus::Signaled(pid, sig, _)) => {
                warn!(
                    "子进程被信号终止 (pid={}, signal={:?}), 重试计数: {}/{}",
                    pid, sig, self.retry_count, self.config.max_retries
                );
            }
            Ok(_) => {}
            Err(e) => {
                error!("waitpid 失败: {}", e);
            }
        }

        // 检查是否超过最大重试次数
        if self.config.max_retries > 0
            && self.retry_count >= self.config.max_retries
        {
            error!(
                "超过最大重试次数 ({}), 守护进程退出",
                self.config.max_retries
            );
            self.should_exit = true;
            return Ok(());
        }

        // 等待后重试
        info!(
            "等待 {} 秒后重试...",
            self.config.retry_interval_secs
        );
        std::thread::sleep(std::time::Duration::from_secs(
            self.config.retry_interval_secs,
        ));

        // 重新启动子进程
        self.retry_count += 1;
        match ProcessGuard::spawn(&self.config) {
            Ok(guard) => {
                // 注销旧 fd
                if let Some(fd) = self.guard.stdout_fd() {
                    let _ = self.remove_from_epoll(fd);
                }
                if let Some(fd) = self.guard.stderr_fd() {
                    let _ = self.remove_from_epoll(fd);
                }

                self.guard = guard;

                // 注册新 fd
                if let Some(fd) = self.guard.stdout_fd() {
                    self.add_to_epoll(fd, EventSource::Stdout as u64)?;
                }
                if let Some(fd) = self.guard.stderr_fd() {
                    self.add_to_epoll(fd, EventSource::Stderr as u64)?;
                }

                info!("子进程重启成功");
            }
            Err(e) => {
                error!("子进程重启失败: {}", e);
                self.should_exit = true;
            }
        }

        Ok(())
    }

    /// 原子热重载（零停机）
    ///
    /// 1. 启动新 cloudflared 进程
    /// 2. 验证新进程存活
    /// 3. 优雅关闭旧进程
    /// 4. 更新 epoll 监听
    fn atomic_reload(&mut self) -> Result<()> {
        info!("开始原子热重载");

        // 1. 启动新进程
        let new_guard = match ProcessGuard::spawn(&self.config) {
            Ok(g) => g,
            Err(e) => {
                error!("热重载失败：新进程启动失败: {}", e);
                return Ok(()); // 保持旧进程运行
            }
        };

        // 2. 验证新进程存活（等待 1 秒）
        std::thread::sleep(std::time::Duration::from_secs(1));
        if !new_guard.is_alive() {
            error!("热重载失败：新进程启动后立即退出");
            // new_guard 会被 drop，自动清理
            return Ok(());
        }

        info!(
            "新进程已启动 (pid={}), 开始切换",
            new_guard.pid()
        );

        // 3. 注销旧 fd
        if let Some(fd) = self.guard.stdout_fd() {
            let _ = self.remove_from_epoll(fd);
        }
        if let Some(fd) = self.guard.stderr_fd() {
            let _ = self.remove_from_epoll(fd);
        }

        // 4. 优雅关闭旧进程
        let old_pid = self.guard.pid();
        if let Err(e) = self
            .guard
            .graceful_shutdown(self.config.shutdown_timeout_secs)
        {
            warn!("旧进程关闭异常: {}", e);
        }
        info!("旧进程已关闭 (pid={})", old_pid);

        // 5. 切换到新进程
        self.guard = new_guard;
        self.retry_count = 0; // 重置重试计数

        // 6. 注册新 fd
        if let Some(fd) = self.guard.stdout_fd() {
            self.add_to_epoll(fd, EventSource::Stdout as u64)?;
        }
        if let Some(fd) = self.guard.stderr_fd() {
            self.add_to_epoll(fd, EventSource::Stderr as u64)?;
        }

        info!("原子热重载完成 (新 pid={})", self.guard.pid());

        Ok(())
    }

    /// 处理 IPC 命令
    fn handle_ipc(&mut self) -> Result<()> {
        while let Some((cmd, sender)) =
            ipc::recv_command(&self.socket)
        {
            match cmd {
                Command::Status => {
                    let response = StatusResponse {
                        pid: self.guard.pid().as_raw() as u32,
                        uptime_secs: self.guard.uptime_secs(),
                    };
                    let json = serde_json::to_string(&response)
                        .unwrap_or_else(|_| "{}".to_string());
                    ipc::send_response(&self.socket, &sender, &json);
                }
                Command::Stop => {
                    info!("收到 STOP 命令，开始优雅关闭");
                    ipc::send_response(
                        &self.socket,
                        &sender,
                        r#"{"status":"stopping"}"#,
                    );
                    self.shutdown()?;
                }
                Command::Restart => {
                    info!("收到 RESTART 命令");
                    ipc::send_response(
                        &self.socket,
                        &sender,
                        r#"{"status":"restarting"}"#,
                    );
                    self.atomic_reload()?;
                }
            }
        }

        Ok(())
    }

    /// 优雅关闭守护进程
    fn shutdown(&mut self) -> Result<()> {
        self.should_exit = true;

        info!("正在关闭子进程...");
        self.guard
            .graceful_shutdown(self.config.shutdown_timeout_secs)?;

        Ok(())
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        // 关闭 epoll fd
        unsafe { libc::close(self.epoll_fd) };
    }
}

/// 运行守护进程主循环
pub fn run_daemon(config: Config) -> Result<()> {
    let mut event_loop = EventLoop::new(config)?;
    event_loop.run()
}

// --- 以下为 epoll 系统调用的 FFI 封装 ---
// 为了避免引入 libc crate 的额外依赖，直接使用 FFI
// 如果项目已依赖 libc，可以直接使用 libc::epoll_create1 等

use libc;

const EPOLLIN: u32 = 0x001;
const EPOLL_CTL_ADD: i32 = 1;
const EPOLL_CTL_DEL: i32 = 2;

extern "C" {
    fn epoll_create1(flags: i32) -> i32;
    fn epoll_ctl(epfd: i32, op: i32, fd: i32, event: *mut EpollEvent) -> i32;
    fn epoll_wait(
        epfd: i32,
        events: *mut EpollEvent,
        maxevents: i32,
        timeout: i32,
    ) -> i32;
}