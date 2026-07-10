//! 进程管理模块
//!
//! ProcessGuard 负责 cloudflared 子进程的生命周期管理：
//! - 启动：构建命令行参数，通过 pipe 捕获 stdout/stderr
//! - 监控：非阻塞检查存活状态
//! - 关闭：先 SIGTERM 优雅关闭，超时后 SIGKILL 强制终止
//!
//! 使用 std::process::Command 而非 tokio::process，符合极简约束。

use std::os::unix::io::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use log::info;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::config::{cloudflared_bin_path, Config};
use crate::error::{Result, TaskModError};

/// 进程守护器，持有子进程句柄和启动时间
pub struct ProcessGuard {
    child: Child,
    pid: Pid,
    start_time: Instant,
}

impl ProcessGuard {
    /// 启动 cloudflared 子进程
    ///
    /// 构建命令行: cloudflared tunnel run --token <token> --url <url>
    /// stdout/stderr 通过 pipe 捕获，由事件循环转发到父进程 stdout
    pub fn spawn(config: &Config) -> Result<Self> {
        let bin_path = cloudflared_bin_path(&config.version)?;

        let mut cmd = Command::new(&bin_path);
        cmd.args([
            "tunnel",
            "run",
            "--token",
            &config.tunnel.token,
            "--url",
            &config.tunnel.url,
        ]);

        // 通过 pipe 捕获子进程输出，用于零拷贝转发
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(TaskModError::ProcessSpawn)?;
        let pid = Pid::from_raw(child.id() as i32);

        info!("cloudflared 已启动 (pid={})", pid);

        Ok(ProcessGuard {
            child,
            pid,
            start_time: Instant::now(),
        })
    }

    /// 获取子进程 PID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// 获取进程运行时长（秒）
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 获取子进程 stdout 的文件描述符（用于 epoll 监听）
    pub fn stdout_fd(&self) -> Option<i32> {
        self.child.stdout.as_ref().map(|s| s.as_raw_fd())
    }

    /// 获取子进程 stderr 的文件描述符（用于 epoll 监听）
    pub fn stderr_fd(&self) -> Option<i32> {
        self.child.stderr.as_ref().map(|s| s.as_raw_fd())
    }

    /// 非阻塞检查进程是否存活
    pub fn is_alive(&self) -> bool {
        // kill(pid, 0) 不发送信号，仅检查进程是否存在
        signal::kill(self.pid, None).is_ok()
    }

    /// 优雅关闭子进程
    ///
    /// 1. 发送 SIGTERM，给进程机会清理资源
    /// 2. 等待 timeout_secs 秒
    /// 3. 超时后发送 SIGKILL 强制终止
    pub fn graceful_shutdown(&mut self, timeout_secs: u64) -> Result<()> {
        info!("正在关闭 cloudflared (pid={})", self.pid);

        // 先尝试 SIGTERM
        if signal::kill(self.pid, Signal::SIGTERM).is_err() {
            // 进程可能已经退出
            return Ok(());
        }

        let deadline = Instant::now() + std::time::Duration::from_secs(timeout_secs);

        // 等待进程退出
        loop {
            if !self.is_alive() {
                info!("cloudflared 已优雅退出 (pid={})", self.pid);
                return Ok(());
            }

            if Instant::now() >= deadline {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        // 超时，强制 SIGKILL
        info!("SIGTERM 超时，发送 SIGKILL (pid={})", self.pid);
        signal::kill(self.pid, Signal::SIGKILL).map_err(TaskModError::Signal)?;

        // 确保子进程被回收
        let _ = self.child.wait();

        Ok(())
    }
}

impl Drop for ProcessGuard {
    /// 确保子进程在 guard 释放时被清理
    fn drop(&mut self) {
        if self.is_alive() {
            let _ = signal::kill(self.pid, Signal::SIGKILL);
            let _ = self.child.wait();
        }
    }
}