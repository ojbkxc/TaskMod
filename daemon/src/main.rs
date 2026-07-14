//! TaskMod 守护进程 - cloudflared 隧道管理器
//!
//! 极简设计，零停机热重载，自动恢复。
//! 所有 I/O 使用标准库，事件循环基于 epoll。
//!
//! 子命令：
//! - start: 启动守护进程（fork 后台运行）
//! - stop: 优雅关闭守护进程
//! - status: 查询进程状态
//! - restart: 触发热重载

use std::env;
use std::process;

use log::info;

mod config;
mod download;
mod error;
mod ipc;
mod r#loop;
mod process;

use error::Result;
use ipc::{Command, Response};

fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    let result = match args[1].as_str() {
        "start" => cmd_start(),
        "stop" => cmd_stop(),
        "status" => cmd_status(),
        "restart" => cmd_restart(),
        _ => {
            eprintln!("未知命令: {}", args[1]);
            print_usage();
            process::exit(1);
        }
    };

    if let Err(e) = result {
        eprintln!("错误: {}", e);
        process::exit(1);
    }
}

fn print_usage() {
    eprintln!(
        r#"TaskMod - cloudflared 隧道守护进程

用法: taskmod <命令>

命令:
  start     启动守护进程（后台运行）
  stop      优雅关闭守护进程
  status    查询进程状态
  restart   触发热重载（零停机）

配置文件: ~/.taskmod/config.toml 或 TASKMOD_CONFIG 环境变量"#
    );
}

/// 启动守护进程
fn cmd_start() -> Result<()> {
    // 检查是否已有实例在运行
    if let Some(pid) = ipc::check_existing_instance() {
        eprintln!("守护进程已在运行 (pid={})", pid);
        process::exit(1);
    }

    // 加载配置
    let config = config::load_config()?;
    info!("配置加载成功");

    // 确保 cloudflared 二进制可用
    download::ensure_binary(&config.version)?;

    // fork 为守护进程
    daemonize()?;

    // 写入 PID 文件
    ipc::write_pid_file(process::id())?;

    info!("TaskMod 守护进程启动 (pid={})", process::id());

    // 注册清理钩子
    ctrlc::set_handler(|| {
        info!("收到终止信号，清理资源...");
        ipc::cleanup();
        process::exit(0);
    })
    .ok();

    // 进入事件循环
    let result = r#loop::run_daemon(config);

    // 清理
    ipc::cleanup();

    result
}

/// 停止守护进程
fn cmd_stop() -> Result<()> {
    let response = ipc::client_send_command(&Command::Stop)?;
    match response {
        Response::Success(msg) => println!("{}", msg),
        Response::Error(msg) => eprintln!("错误: {}", msg),
        Response::Json(val) => println!("{}", val),
    }
    Ok(())
}

/// 查询状态
fn cmd_status() -> Result<()> {
    let response = ipc::client_send_command(&Command::Status)?;

    match response {
        Response::Success(msg) => println!("{}", msg),
        Response::Error(msg) => eprintln!("错误: {}", msg),
        Response::Json(json) => {
            if let (Some(pid), Some(uptime)) = (
                json.get("pid").and_then(|v| v.as_u64()),
                json.get("uptime_secs").and_then(|v| v.as_u64()),
            ) {
                println!("状态: 运行中");
                println!("PID: {}", pid);
                println!("运行时长: {} 秒", uptime);
            } else {
                println!("{}", json);
            }
        }
    }

    Ok(())
}

/// 触发热重载
fn cmd_restart() -> Result<()> {
    let response = ipc::client_send_command(&Command::RestartAll)?;
    match response {
        Response::Success(msg) => println!("{}", msg),
        Response::Error(msg) => eprintln!("错误: {}", msg),
        Response::Json(val) => println!("{}", val),
    }
    Ok(())
}

/// 将当前进程 fork 为守护进程
///
/// 使用 libc::fork() 实现 Unix 守护进程标准流程：
/// 1. fork() 创建子进程
/// 2. 父进程退出
/// 3. 子进程 setsid() 创建新会话
/// 4. fork() 再次 fork（防止终端重新关联）
/// 5. 切换工作目录到 /
/// 6. 关闭 stdin/stdout/stderr（日志通过文件或 syslog）
fn daemonize() -> Result<()> {
    use nix::unistd::{close, dup2, setsid};
    use std::os::unix::io::RawFd;

    // 第一次 fork
    match unsafe { libc::fork() } {
        -1 => {
            return Err(error::TaskModError::Io(std::io::Error::last_os_error()));
        }
        0 => {} // 子进程继续
        _ => {
            // 父进程退出
            process::exit(0);
        }
    }

    // 创建新会话
    setsid().map_err(error::TaskModError::Signal)?;

    // 第二次 fork（防止终端重新关联）
    match unsafe { libc::fork() } {
        -1 => {
            return Err(error::TaskModError::Io(std::io::Error::last_os_error()));
        }
        0 => {} // 子进程继续
        _ => {
            // 父进程退出
            process::exit(0);
        }
    }

    // 切换工作目录
    nix::unistd::chdir("/").map_err(error::TaskModError::Signal)?;

    // 重定向 stdin/stdout/stderr 到 /dev/null
    let devnull = unsafe { libc::open(b"/dev/null\0".as_ptr() as *const _, libc::O_RDWR) };
    if devnull < 0 {
        return Err(error::TaskModError::Io(std::io::Error::last_os_error()));
    }

    dup2(devnull, 0).map_err(error::TaskModError::Signal)?; // stdin
    dup2(devnull, 1).map_err(error::TaskModError::Signal)?; // stdout
    dup2(devnull, 2).map_err(error::TaskModError::Signal)?; // stderr

    if devnull > 2 {
        close(devnull).map_err(error::TaskModError::Signal)?;
    }

    Ok(())
}