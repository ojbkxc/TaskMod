//! 进程管理模块
//!
//! ProcessGuard 负责单个 cloudflared 子进程的生命周期管理
//! ProcessManager 负责管理多个隧道对应的进程

use std::collections::HashMap;
use std::os::unix::io::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::time::Instant;

use log::info;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;

use crate::config::{cloudflared_bin_path, GlobalConfig, TunnelConfig};
use crate::error::{Result, TaskModError};

/// 进程守护器，持有子进程句柄和启动时间
pub struct ProcessGuard {
    child: Child,
    pid: Pid,
    start_time: Instant,
    tunnel_name: String,
}

/// 进程状态信息
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessStatus {
    pub tunnel_name: String,
    pub pid: u32,
    pub uptime_secs: u64,
    pub is_alive: bool,
}

impl ProcessGuard {
    /// 启动 cloudflared 子进程
    ///
    /// 每个隧道使用一个独立的 cloudflared 进程
    pub fn spawn(
        global: &GlobalConfig,
        tunnel: &TunnelConfig,
    ) -> Result<Self> {
        let bin_path = cloudflared_bin_path(&global.version)?;

        // 构建服务 URL 列表
        let active_services: Vec<&str> = tunnel
            .services
            .iter()
            .filter(|s| s.enabled)
            .map(|s| s.url.as_str())
            .collect();

        if active_services.is_empty() {
            return Err(TaskModError::ConfigMissingField(
                format!("隧道 '{}' 没有启用的服务", tunnel.name),
            ));
        }

        let mut cmd = Command::new(&bin_path);
        cmd.arg("tunnel").arg("run");

        // 添加 token
        cmd.arg("--token").arg(&tunnel.token);

        // 添加服务 URL
        for url in &active_services {
            cmd.arg("--url").arg(url);
        }

        // 通过 pipe 捕获子进程输出
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let child = cmd.spawn().map_err(TaskModError::ProcessSpawn)?;
        let pid = Pid::from_raw(child.id() as i32);

        info!(
            "隧道 '{}' 的 cloudflared 已启动 (pid={})",
            tunnel.name, pid
        );

        Ok(ProcessGuard {
            child,
            pid,
            start_time: Instant::now(),
            tunnel_name: tunnel.name.clone(),
        })
    }

    /// 获取子进程 PID
    pub fn pid(&self) -> Pid {
        self.pid
    }

    /// 获取隧道名称
    pub fn tunnel_name(&self) -> &str {
        &self.tunnel_name
    }

    /// 获取进程运行时长（秒）
    pub fn uptime_secs(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// 获取子进程 stdout 的文件描述符
    pub fn stdout_fd(&self) -> Option<i32> {
        self.child.stdout.as_ref().map(|s| s.as_raw_fd())
    }

    /// 获取子进程 stderr 的文件描述符
    pub fn stderr_fd(&self) -> Option<i32> {
        self.child.stderr.as_ref().map(|s| s.as_raw_fd())
    }

    /// 非阻塞检查进程是否存活
    pub fn is_alive(&self) -> bool {
        signal::kill(self.pid, None).is_ok()
    }

    /// 获取进程状态
    pub fn status(&self) -> ProcessStatus {
        ProcessStatus {
            tunnel_name: self.tunnel_name.clone(),
            pid: self.pid.as_raw() as u32,
            uptime_secs: self.uptime_secs(),
            is_alive: self.is_alive(),
        }
    }

    /// 优雅关闭子进程
    pub fn graceful_shutdown(&mut self, timeout_secs: u64) -> Result<()> {
        info!(
            "正在关闭隧道 '{}' 的 cloudflared (pid={})",
            self.tunnel_name, self.pid
        );

        if signal::kill(self.pid, Signal::SIGTERM).is_err() {
            return Ok(());
        }

        let deadline = Instant::now() + std::time::Duration::from_secs(timeout_secs);

        loop {
            if !self.is_alive() {
                info!(
                    "隧道 '{}' 的 cloudflared 已优雅退出 (pid={})",
                    self.tunnel_name, self.pid
                );
                return Ok(());
            }

            if Instant::now() >= deadline {
                break;
            }

            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        info!(
            "SIGTERM 超时，发送 SIGKILL (pid={})",
            self.pid
        );
        signal::kill(self.pid, Signal::SIGKILL).map_err(TaskModError::Signal)?;
        let _ = self.child.wait();

        Ok(())
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        if self.is_alive() {
            let _ = signal::kill(self.pid, Signal::SIGKILL);
            let _ = self.child.wait();
        }
    }
}

/// 进程管理器，管理多个隧道的进程
pub struct ProcessManager {
    processes: HashMap<String, ProcessGuard>,
}

impl ProcessManager {
    /// 创建新的进程管理器
    pub fn new() -> Self {
        ProcessManager {
            processes: HashMap::new(),
        }
    }

    /// 获取所有进程状态
    pub fn list_status(&self) -> Vec<ProcessStatus> {
        self.processes.values().map(|p| p.status()).collect()
    }

    /// 获取指定隧道的进程状态
    pub fn get_status(&self, tunnel_name: &str) -> Option<ProcessStatus> {
        self.processes.get(tunnel_name).map(|p| p.status())
    }

    /// 检查隧道进程是否在运行
    pub fn is_running(&self, tunnel_name: &str) -> bool {
        self.processes
            .get(tunnel_name)
            .map(|p| p.is_alive())
            .unwrap_or(false)
    }

    /// 启动隧道进程
    pub fn start_tunnel(
        &mut self,
        global: &GlobalConfig,
        tunnel: &TunnelConfig,
    ) -> Result<()> {
        if self.is_running(&tunnel.name) {
            info!("隧道 '{}' 已在运行", tunnel.name);
            return Ok(());
        }

        let guard = ProcessGuard::spawn(global, tunnel)?;
        self.processes.insert(tunnel.name.clone(), guard);

        Ok(())
    }

    /// 停止隧道进程
    pub fn stop_tunnel(
        &mut self,
        tunnel_name: &str,
        timeout_secs: u64,
    ) -> Result<()> {
        if let Some(mut guard) = self.processes.remove(tunnel_name) {
            guard.graceful_shutdown(timeout_secs)?;
            info!("隧道 '{}' 已停止", tunnel_name);
        }
        Ok(())
    }

    /// 重启隧道进程（热重载）
    pub fn restart_tunnel(
        &mut self,
        global: &GlobalConfig,
        tunnel: &TunnelConfig,
        timeout_secs: u64,
    ) -> Result<()> {
        info!("重启隧道 '{}'", tunnel.name);

        // 先停止旧进程
        if self.is_running(&tunnel.name) {
            self.stop_tunnel(&tunnel.name, timeout_secs)?;
        }

        // 启动新进程
        self.start_tunnel(global, tunnel)?;

        info!("隧道 '{}' 重启完成", tunnel.name);
        Ok(())
    }

    /// 停止所有进程
    pub fn stop_all(&mut self, timeout_secs: u64) -> Result<()> {
        let names: Vec<String> = self.processes.keys().cloned().collect();
        for name in names {
            self.stop_tunnel(&name, timeout_secs)?;
        }
        Ok(())
    }

    /// 获取进程的 stdout fd（用于 epoll 监听）
    pub fn stdout_fd(&self, tunnel_name: &str) -> Option<i32> {
        self.processes.get(tunnel_name).and_then(|p| p.stdout_fd())
    }

    /// 获取进程的 stderr fd（用于 epoll 监听）
    pub fn stderr_fd(&self, tunnel_name: &str) -> Option<i32> {
        self.processes.get(tunnel_name).and_then(|p| p.stderr_fd())
    }

    /// 处理子进程退出
    pub fn handle_child_exit(&mut self, pid: Pid) -> Option<String> {
        let tunnel_name = self
            .processes
            .iter()
            .find(|(_, p)| p.pid() == pid)
            .map(|(name, _)| name.clone());

        if let Some(name) = &tunnel_name {
            self.processes.remove(name);
        }

        tunnel_name
    }

    /// 获取所有运行中的隧道名称
    pub fn running_tunnels(&self) -> Vec<String> {
        self.processes
            .iter()
            .filter(|(_, p)| p.is_alive())
            .map(|(name, _)| name.clone())
            .collect()
    }
}