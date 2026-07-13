//! 事件循环核心模块
//!
//! 基于 epoll 的事件多路复用，监听以下事件源：
//! - 子进程 stdout pipe → 转发到父进程 stdout
//! - 子进程 stderr pipe → 转发到父进程 stderr
//! - Unix socket → 处理 IPC 命令
//! - signalfd → 处理 SIGCHLD/SIGUSR1 信号
//!
//! 支持多隧道管理，使用 ProcessManager 管理所有隧道进程。

use std::io::{self, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::Instant;

use log::{error, info, warn};
use nix::sys::signal::{self, SigSet, Signal};
use nix::sys::signalfd::{SignalFd, SfdFlags};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::Pid;

use crate::config::Config;
use crate::download::{download_binary, get_target_arch, is_binary_available};
use crate::error::{Result, TaskModError};
use crate::ipc::{self, Command, Response};
use crate::process::{ProcessManager, ProcessStatus};

#[derive(Debug, Clone, Copy, PartialEq)]
enum EventSource {
    Stdout(u64),
    Stderr(u64),
    Signal,
    Ipc,
}

struct EventLoop {
    config: Config,
    manager: ProcessManager,
    socket: std::os::unix::net::UnixDatagram,
    signal_fd: SignalFd,
    epoll_fd: RawFd,
    retry_count: u32,
    should_exit: bool,
}

#[repr(C)]
struct EpollEvent {
    events: u32,
    data: u64,
}

impl EventSource {
    fn from_u64(v: u64) -> Option<Self> {
        if v == EventSource::Signal as u64 {
            Some(EventSource::Signal)
        } else if v == EventSource::Ipc as u64 {
            Some(EventSource::Ipc)
        } else if (v & 0xFFFF0000_00000000) == (EventSource::Stdout(0) as u64 & 0xFFFF0000_00000000) {
            Some(EventSource::Stdout(v & 0x0000FFFF_FFFFFFFF))
        } else if (v & 0xFFFF0000_00000000) == (EventSource::Stderr(0) as u64 & 0xFFFF0000_00000000) {
            Some(EventSource::Stderr(v & 0x0000FFFF_FFFFFFFF))
        } else {
            None
        }
    }

    fn as_u64(&self) -> u64 {
        match self {
            EventSource::Stdout(id) => 0x00010000_00000000 | id,
            EventSource::Stderr(id) => 0x00020000_00000000 | id,
            EventSource::Signal => 0x00030000_00000000,
            EventSource::Ipc => 0x00040000_00000000,
        }
    }
}

impl EventLoop {
    fn new(config: Config) -> Result<Self> {
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

        let socket = ipc::create_socket()?;

        let epoll_fd = unsafe { epoll_create1(0) };
        if epoll_fd < 0 {
            return Err(TaskModError::Io(io::Error::last_os_error()));
        }

        let mut loop_state = EventLoop {
            config: config.clone(),
            manager: ProcessManager::new(),
            socket,
            signal_fd,
            epoll_fd,
            retry_count: 0,
            should_exit: false,
        };

        loop_state.add_to_epoll(
            loop_state.signal_fd.as_raw_fd(),
            EventSource::Signal.as_u64(),
        )?;
        loop_state.add_to_epoll(
            loop_state.socket.as_raw_fd(),
            EventSource::Ipc.as_u64(),
        )?;

        for tunnel in &config.tunnels {
            if tunnel.enabled {
                if let Err(e) = loop_state.start_tunnel(tunnel) {
                    warn!("启动隧道 '{}' 失败: {}", tunnel.name, e);
                }
            }
        }

        Ok(loop_state)
    }

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
            let err = io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::ENOENT) {
                return Err(TaskModError::Io(err));
            }
        }

        Ok(())
    }

    fn start_tunnel(&mut self, tunnel: &crate::config::TunnelConfig) -> Result<()> {
        if self.manager.is_running(&tunnel.name) {
            return Ok(());
        }

        self.manager.start_tunnel(&self.config, tunnel)?;
        info!("隧道 '{}' 已启动", tunnel.name);

        Ok(())
    }

    fn run(&mut self) -> Result<()> {
        info!("事件循环启动");

        let mut events = [EpollEvent { events: 0, data: 0 }; 32];

        while !self.should_exit {
            let nfds = unsafe {
                epoll_wait(self.epoll_fd, events.as_mut_ptr(), 32, 100)
            };

            if nfds < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(TaskModError::Io(err));
            }

            for i in 0..nfds as usize {
                if let Some(source) = EventSource::from_u64(events[i].data) {
                    match source {
                        EventSource::Stdout(_) => self.handle_stdout()?,
                        EventSource::Stderr(_) => self.handle_stderr()?,
                        EventSource::Signal => self.handle_signal()?,
                        EventSource::Ipc => self.handle_ipc()?,
                    }
                }
            }

            self.check_child_exit();
        }

        info!("事件循环退出");
        Ok(())
    }

    fn handle_stdout(&mut self) -> Result<()> {
        for name in self.manager.running_tunnels() {
            if let Some(fd) = self.manager.stdout_fd(&name) {
                self.forward_pipe_output(fd, io::stdout())?;
            }
        }
        Ok(())
    }

    fn handle_stderr(&mut self) -> Result<()> {
        for name in self.manager.running_tunnels() {
            if let Some(fd) = self.manager.stderr_fd(&name) {
                self.forward_pipe_output(fd, io::stderr())?;
            }
        }
        Ok(())
    }

    fn forward_pipe_output<W: Write>(
        &self,
        fd: RawFd,
        mut writer: W,
    ) -> Result<()> {
        let mut buf = [0u8; 8192];
        loop {
            let n = unsafe { libc::read(fd, buf.as_mut_ptr() as *mut _, buf.len()) };
            if n <= 0 {
                if n < 0 {
                    let err = io::Error::last_os_error();
                    if err.kind() == io::ErrorKind::WouldBlock {
                        break;
                    }
                    return Err(TaskModError::Io(err));
                }
                break;
            }

            writer.write_all(&buf[..n]).map_err(TaskModError::Io)?;
            writer.flush().map_err(TaskModError::Io)?;
        }

        Ok(())
    }

    fn handle_signal(&mut self) -> Result<()> {
        match self.signal_fd.read_signal() {
            Ok(Some(siginfo)) => {
                match Signal::try_from(siginfo.ssi_signo as i32) {
                    Ok(Signal::SIGCHLD) => {
                        info!("收到 SIGCHLD 信号");
                        self.check_child_exit();
                    }
                    Ok(Signal::SIGUSR1) => {
                        info!("收到 SIGUSR1 信号，触发热重载");
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

    fn check_child_exit(&mut self) {
        let mut exited_pids = Vec::new();
        for name in self.manager.running_tunnels() {
            if let Some(status) = self.manager.get_status(&name) {
                if !status.is_alive {
                    if let Some(pid) = self.manager.get_status(&name).map(|s| s.pid) {
                        exited_pids.push((name, pid));
                    }
                }
            }
        }

        for (name, pid) in exited_pids {
            info!("隧道 '{}' 的进程已退出 (pid={})", name, pid);
            self.manager.handle_child_exit(Pid::from_raw(pid as i32));

            if self.config.max_retries == 0 || self.retry_count < self.config.max_retries {
                info!("等待 {} 秒后重试隧道 '{}'...", self.config.retry_interval_secs, name);
                let _ = std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                });
                self.retry_count += 1;
            }
        }
    }

    fn handle_ipc(&mut self) -> Result<()> {
        while let Some((cmd, sender)) = ipc::recv_command(&self.socket) {
            match cmd {
                Command::Status => {
                    let status: serde_json::Value = serde_json::json!({
                        "running": true,
                        "tunnels": self.manager.list_status(),
                    });
                    ipc::send_json(&self.socket, &sender, status);
                }
                Command::Stop => {
                    info!("收到 STOP 命令，开始优雅关闭");
                    ipc::send_success(&self.socket, &sender, "正在停止...");
                    self.shutdown()?;
                }
                Command::RestartAll => {
                    info!("收到 RESTART 命令");
                    ipc::send_success(&self.socket, &sender, "正在重启...");
                    self.retry_count = 0;
                    for tunnel in &self.config.tunnels {
                        if tunnel.enabled {
                            let _ = self.manager.restart_tunnel(
                                &self.config,
                                tunnel,
                                self.config.shutdown_timeout_secs,
                            );
                        }
                    }
                }
                Command::GetCloudflaredStatus => {
                    let arch = get_target_arch();
                    let version = self.config.global.version.clone();
                    let available = is_binary_available(&version).unwrap_or(false);
                    let status: serde_json::Value = serde_json::json!({
                        "version": version,
                        "arch": arch,
                        "available": available,
                    });
                    ipc::send_json(&self.socket, &sender, status);
                }
                Command::DownloadCloudflared { version } => {
                    info!("收到下载命令，版本: {}", version);
                    match download_binary(&version) {
                        Ok(_) => {
                            ipc::send_success(&self.socket, &sender, &format!("cloudflared {} 下载完成", version));
                        }
                        Err(e) => {
                            ipc::send_error(&self.socket, &sender, &format!("下载失败: {}", e));
                        }
                    }
                }
                Command::ListCloudflaredVersions => {
                    let versions = vec![
                        "2026.7.1",
                        "2026.7.0",
                        "2026.6.1",
                        "2026.6.0",
                        "2026.5.0",
                        "2026.4.0",
                        "2026.3.0",
                        "2026.2.0",
                        "2025.12.0",
                        "2025.10.0",
                        "2025.8.0",
                        "2025.6.0",
                        "2025.4.0",
                        "2025.2.0",
                        "2024.10.1",
                    ];
                    let json = serde_json::to_value(versions)
                        .unwrap_or(serde_json::Value::Null);
                    ipc::send_json(&self.socket, &sender, json);
                }
                Command::ListTunnels => {
                    let tunnels: serde_json::Value = serde_json::to_value(&self.config.tunnels)
                        .unwrap_or(serde_json::Value::Null);
                    ipc::send_json(&self.socket, &sender, tunnels);
                }
                Command::GetTunnel { name } => {
                    if let Some(tunnel) = self.config.tunnels.iter().find(|t| t.name == name) {
                        let json = serde_json::to_value(tunnel)
                            .unwrap_or(serde_json::Value::Null);
                        ipc::send_json(&self.socket, &sender, json);
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", name));
                    }
                }
                Command::AddTunnel { name, token, enabled } => {
                    let tunnel = crate::config::TunnelConfig {
                        name: name.clone(),
                        token,
                        enabled,
                        services: Vec::new(),
                    };
                    let mut config = self.config.clone();
                    config.tunnels.push(tunnel);
                    if let Err(e) = crate::config::save_config(&config) {
                        ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                    } else {
                        self.config = config;
                        ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 添加成功", name));
                    }
                }
                Command::DeleteTunnel { name } => {
                    let mut config = self.config.clone();
                    let index = config.tunnels.iter().position(|t| t.name == name);
                    if let Some(i) = index {
                        config.tunnels.remove(i);
                        if let Err(e) = crate::config::save_config(&config) {
                            ipc::send_error(&self.socket, &sender, &format!("删除失败: {}", e));
                        } else {
                            self.config = config;
                            let _ = self.manager.stop_tunnel(&name, self.config.global.shutdown_timeout_secs);
                            ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 删除成功", name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", name));
                    }
                }
                Command::EnableTunnel { name } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == name) {
                        tunnel.enabled = true;
                        if let Err(e) = crate::config::save_config(&config) {
                            ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                        } else {
                            self.config = config;
                            ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 已启用", name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", name));
                    }
                }
                Command::DisableTunnel { name } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == name) {
                        tunnel.enabled = false;
                        if let Err(e) = crate::config::save_config(&config) {
                            ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                        } else {
                            self.config = config;
                            let _ = self.manager.stop_tunnel(&name, self.config.global.shutdown_timeout_secs);
                            ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 已禁用", name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", name));
                    }
                }
                Command::StartTunnel { name } => {
                    if let Some(tunnel) = self.config.tunnels.iter().find(|t| t.name == name) {
                        match self.start_tunnel(tunnel) {
                            Ok(_) => ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 已启动", name)),
                            Err(e) => ipc::send_error(&self.socket, &sender, &format!("启动失败: {}", e)),
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", name));
                    }
                }
                Command::StopTunnel { name } => {
                    match self.manager.stop_tunnel(&name, self.config.global.shutdown_timeout_secs) {
                        Ok(_) => ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 已停止", name)),
                        Err(e) => ipc::send_error(&self.socket, &sender, &format!("停止失败: {}", e)),
                    }
                }
                Command::RestartTunnel { name } => {
                    if let Some(tunnel) = self.config.tunnels.iter().find(|t| t.name == name) {
                        match self.manager.restart_tunnel(&self.config, tunnel, self.config.global.shutdown_timeout_secs) {
                            Ok(_) => ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 已重启", name)),
                            Err(e) => ipc::send_error(&self.socket, &sender, &format!("重启失败: {}", e)),
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", name));
                    }
                }
                Command::ListServices { tunnel_name } => {
                    if let Some(tunnel) = self.config.tunnels.iter().find(|t| t.name == tunnel_name) {
                        let services: serde_json::Value = serde_json::to_value(&tunnel.services)
                            .unwrap_or(serde_json::Value::Null);
                        ipc::send_json(&self.socket, &sender, services);
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", tunnel_name));
                    }
                }
                Command::AddService { tunnel_name, service_name, url, enabled } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == tunnel_name) {
                        let service = crate::config::ServiceConfig {
                            name: service_name.clone(),
                            url,
                            enabled,
                        };
                        tunnel.services.push(service);
                        if let Err(e) = crate::config::save_config(&config) {
                            ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                        } else {
                            self.config = config;
                            ipc::send_success(&self.socket, &sender, &format!("服务 '{}' 添加成功", service_name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", tunnel_name));
                    }
                }
                Command::DeleteService { tunnel_name, service_name } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == tunnel_name) {
                        let index = tunnel.services.iter().position(|s| s.name == service_name);
                        if let Some(i) = index {
                            tunnel.services.remove(i);
                            if let Err(e) = crate::config::save_config(&config) {
                                ipc::send_error(&self.socket, &sender, &format!("删除失败: {}", e));
                            } else {
                                self.config = config;
                                ipc::send_success(&self.socket, &sender, &format!("服务 '{}' 删除成功", service_name));
                            }
                        } else {
                            ipc::send_error(&self.socket, &sender, &format!("服务 '{}' 不存在", service_name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", tunnel_name));
                    }
                }
                Command::EnableService { tunnel_name, service_name } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == tunnel_name) {
                        if let Some(service) = tunnel.services.iter_mut().find(|s| s.name == service_name) {
                            service.enabled = true;
                            if let Err(e) = crate::config::save_config(&config) {
                                ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                            } else {
                                self.config = config;
                                ipc::send_success(&self.socket, &sender, &format!("服务 '{}' 已启用", service_name));
                            }
                        } else {
                            ipc::send_error(&self.socket, &sender, &format!("服务 '{}' 不存在", service_name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", tunnel_name));
                    }
                }
                Command::DisableService { tunnel_name, service_name } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == tunnel_name) {
                        if let Some(service) = tunnel.services.iter_mut().find(|s| s.name == service_name) {
                            service.enabled = false;
                            if let Err(e) = crate::config::save_config(&config) {
                                ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                            } else {
                                self.config = config;
                                ipc::send_success(&self.socket, &sender, &format!("服务 '{}' 已禁用", service_name));
                            }
                        } else {
                            ipc::send_error(&self.socket, &sender, &format!("服务 '{}' 不存在", service_name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", tunnel_name));
                    }
                }
                Command::ListProcesses => {
                    let status: serde_json::Value = serde_json::to_value(&self.manager.list_status())
                        .unwrap_or(serde_json::Value::Null);
                    ipc::send_json(&self.socket, &sender, status);
                }
                Command::GetProcessStatus { tunnel_name } => {
                    if let Some(status) = self.manager.get_status(&tunnel_name) {
                        let json = serde_json::to_value(status)
                            .unwrap_or(serde_json::Value::Null);
                        ipc::send_json(&self.socket, &sender, json);
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 未运行", tunnel_name));
                    }
                }
                Command::UpdateTunnel { name, new_name, token, enabled } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == name) {
                        if let Some(nn) = new_name {
                            tunnel.name = nn;
                        }
                        if let Some(t) = token {
                            tunnel.token = t;
                        }
                        if let Some(e) = enabled {
                            tunnel.enabled = e;
                        }
                        if let Err(e) = crate::config::save_config(&config) {
                            ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                        } else {
                            self.config = config;
                            ipc::send_success(&self.socket, &sender, &format!("隧道 '{}' 更新成功", name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", name));
                    }
                }
                Command::GetService { tunnel_name, service_name } => {
                    if let Some(tunnel) = self.config.tunnels.iter().find(|t| t.name == tunnel_name) {
                        if let Some(service) = tunnel.services.iter().find(|s| s.name == service_name) {
                            let json = serde_json::to_value(service)
                                .unwrap_or(serde_json::Value::Null);
                            ipc::send_json(&self.socket, &sender, json);
                        } else {
                            ipc::send_error(&self.socket, &sender, &format!("服务 '{}' 不存在", service_name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", tunnel_name));
                    }
                }
                Command::UpdateService { tunnel_name, service_name, new_name, url, enabled } => {
                    let mut config = self.config.clone();
                    if let Some(tunnel) = config.tunnels.iter_mut().find(|t| t.name == tunnel_name) {
                        if let Some(service) = tunnel.services.iter_mut().find(|s| s.name == service_name) {
                            if let Some(nn) = new_name {
                                service.name = nn;
                            }
                            if let Some(u) = url {
                                service.url = u;
                            }
                            if let Some(e) = enabled {
                                service.enabled = e;
                            }
                            if let Err(e) = crate::config::save_config(&config) {
                                ipc::send_error(&self.socket, &sender, &format!("保存失败: {}", e));
                            } else {
                                self.config = config;
                                ipc::send_success(&self.socket, &sender, &format!("服务 '{}' 更新成功", service_name));
                            }
                        } else {
                            ipc::send_error(&self.socket, &sender, &format!("服务 '{}' 不存在", service_name));
                        }
                    } else {
                        ipc::send_error(&self.socket, &sender, &format!("隧道 '{}' 不存在", tunnel_name));
                    }
                }
            }
        }

        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.should_exit = true;

        info!("正在关闭所有隧道...");
        self.manager.stop_all(self.config.global.shutdown_timeout_secs)?;

        Ok(())
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        unsafe { libc::close(self.epoll_fd) };
    }
}

pub fn run_daemon(config: Config) -> Result<()> {
    let mut event_loop = EventLoop::new(config)?;
    event_loop.run()
}

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
