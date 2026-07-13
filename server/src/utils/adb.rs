use tokio::process::Command;
use tracing::{info, warn};

/// Android 系统命令绝对路径常量
const SH: &str = "/system/bin/sh";
const INPUT: &str = "/system/bin/input";
const WM: &str = "/system/bin/wm";
const AM: &str = "/system/bin/am";
const PM: &str = "/system/bin/pm";
const CMD: &str = "/system/bin/cmd";
const DUMPSYS: &str = "/system/bin/dumpsys";
const GETPROP: &str = "/system/bin/getprop";
const DF: &str = "/system/bin/df";
const PS: &str = "/system/bin/ps";
const MONKEY: &str = "/system/bin/monkey";
const SCREENCAP: &str = "/system/bin/screencap";
const REBOOT: &str = "/system/bin/reboot";

/// 解析命令输出，返回成功/失败及完整信息
fn parse_output(o: &std::process::Output, success_msg: &str, fail_msg: &str) -> String {
    let stdout = String::from_utf8_lossy(&o.stdout);
    let stderr = String::from_utf8_lossy(&o.stderr);
    if o.status.success() {
        if stdout.trim().is_empty() {
            success_msg.to_string()
        } else {
            format!("{}\n{}", success_msg, stdout)
        }
    } else {
        let err_detail = if !stderr.trim().is_empty() {
            stderr.trim().to_string()
        } else if !stdout.trim().is_empty() {
            stdout.trim().to_string()
        } else {
            format!("退出码: {}", o.status.code().unwrap_or(-1))
        };
        format!("{}: {}", fail_msg, err_detail)
    }
}

/// 执行shell命令（通过sh -c，支持管道、引号等复杂语法）
pub async fn run_command(cmd: &str) -> Result<String, String> {
    if cmd.trim().is_empty() {
        return Err("命令为空".to_string());
    }

    info!("[adb] run_command: {}", cmd);
    match Command::new(SH).arg("-c").arg(cmd).output().await {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                Ok(stdout.to_string())
            } else {
                Err(format!("{}\nstderr: {}", stdout, stderr))
            }
        }
        Err(e) => Err(format!("命令执行失败: {}", e)),
    }
}

/// 执行命令并返回原始Output
#[allow(dead_code)]
pub async fn run_command_raw(cmd: &str) -> Result<std::process::Output, String> {
    if cmd.trim().is_empty() {
        return Err("命令为空".to_string());
    }

    Command::new(SH)
        .arg("-c")
        .arg(cmd)
        .output()
        .await
        .map_err(|e| format!("命令执行失败: {}", e))
}

/// 执行命令（参数列表方式，适合精确控制参数）
pub async fn execute_command(cmd_parts: &[String]) -> Result<std::process::Output, String> {
    if cmd_parts.is_empty() {
        return Err("命令为空".to_string());
    }

    let mut command = Command::new(&cmd_parts[0]);
    for part in cmd_parts.iter().skip(1) {
        command.arg(part);
    }

    command.output().await.map_err(|e| format!("命令执行失败: {}", e))
}

pub async fn get_screen_size() -> String {
    match Command::new(WM).arg("size").output().await {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            // 优先取 Override size（用户自定义分辨率），否则取 Physical size
            for line in output.lines() {
                if line.contains("Override size:") {
                    return line.replace("Override size:", "").trim().to_string();
                }
            }
            for line in output.lines() {
                if line.contains("Physical size:") {
                    return line.replace("Physical size:", "").trim().to_string();
                }
            }
            output.trim().to_string()
        }
        Err(_) => "unknown".to_string(),
    }
}

pub async fn get_wifi_info() -> String {
    match Command::new(DUMPSYS).arg("wifi").output().await {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            let ssid = output.lines().find(|l| l.contains("SSID:")).unwrap_or("");
            let bssid = output.lines().find(|l| l.contains("BSSID:")).unwrap_or("");
            let ip = output.lines().find(|l| l.contains("IP address:")).unwrap_or("");
            format!("WiFi信息:\n{}\n{}\n{}", ssid, bssid, ip)
        }
        Err(e) => format!("获取WiFi信息失败: {}", e),
    }
}

pub async fn get_battery_info() -> String {
    match Command::new(DUMPSYS).arg("battery").output().await {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            let lines: Vec<&str> = output.lines()
                .filter(|l| l.contains(": "))
                .take(10)
                .collect();
            format!("电池信息:\n{}", lines.join("\n"))
        }
        Err(e) => format!("获取电池信息失败: {}", e),
    }
}

pub async fn get_device_info() -> String {
    let mut info = String::new();

    if let Ok(o) = Command::new(GETPROP).arg("ro.product.model").output().await {
        info.push_str(&format!("设备型号: {}\n", String::from_utf8_lossy(&o.stdout).trim()));
    }

    if let Ok(o) = Command::new(GETPROP).arg("ro.build.version.release").output().await {
        info.push_str(&format!("Android版本: {}\n", String::from_utf8_lossy(&o.stdout).trim()));
    }

    if let Ok(o) = Command::new(DF).arg("-h").output().await {
        info.push_str(&format!("存储信息:\n{}\n", String::from_utf8_lossy(&o.stdout)));
    }

    if info.is_empty() {
        "获取设备信息失败".to_string()
    } else {
        info
    }
}

pub async fn get_running_apps() -> String {
    match Command::new(PS).arg("-A").output().await {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            let apps: Vec<&str> = output.lines()
                .filter(|l| l.contains("com."))
                .filter(|l| !l.contains("system_server"))
                .take(20)
                .collect();
            format!("运行中的应用:\n{}", apps.join("\n"))
        }
        Err(e) => format!("获取应用列表失败: {}", e),
    }
}

pub async fn start_app(package_name: &str) -> String {
    // 先用 cmd package resolve-activity 解析出主Activity名
    if let Ok(o) = Command::new(SH)
        .arg("-c")
        .arg(format!("{} package resolve-activity --brief {} | tail -1", CMD, package_name))
        .output()
        .await
    {
        let activity = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if !activity.is_empty() && !activity.contains("Error") && activity.contains('/') {
            // 成功解析到Activity，用 am start -n 启动
            match Command::new(AM)
                .arg("start")
                .arg("-n")
                .arg(&activity)
                .output()
                .await
            {
                Ok(o) => {
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    if !stderr.contains("Error") && !stderr.contains("error") {
                        return format!("应用启动成功: {}", activity);
                    }
                }
                Err(_) => {}
            }
        }
    }

    // fallback: 用 monkey 启动
    match Command::new(MONKEY)
        .arg("-p")
        .arg(package_name)
        .arg("-c")
        .arg("android.intent.category.LAUNCHER")
        .arg("1")
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("应用启动成功: {}", stdout.trim())
            } else {
                format!("应用启动失败: {}", stderr.trim())
            }
        }
        Err(e) => format!("应用启动失败: {}", e),
    }
}

pub async fn stop_app(package_name: &str) -> String {
    match Command::new(AM)
        .arg("force-stop")
        .arg(package_name)
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("应用已停止: {}", package_name)
            } else {
                format!("停止应用失败: {}\nstderr: {}", package_name, stderr.trim())
            }
        }
        Err(e) => format!("停止应用失败: {}", e),
    }
}

pub async fn clear_app_data(package_name: &str) -> String {
    match Command::new(PM)
        .arg("clear")
        .arg(package_name)
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("数据清除成功: {}", stdout.trim())
            } else {
                format!("数据清除失败: {} (可能需要root权限)\nstderr: {}", package_name, stderr.trim())
            }
        }
        Err(e) => format!("清除数据失败: {}", e),
    }
}

pub async fn tap(x: i32, y: i32) -> String {
    info!("[adb] tap: ({}, {})", x, y);
    match Command::new(INPUT)
        .arg("tap")
        .arg(x.to_string())
        .arg(y.to_string())
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("点击成功: ({}, {})", x, y)
            } else {
                let err = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    format!("退出码: {}", o.status.code().unwrap_or(-1))
                };
                warn!("[adb] tap 失败: ({}, {}) - {}", x, y, err);
                format!("点击失败 ({}, {}): {}", x, y, err)
            }
        }
        Err(e) => {
            warn!("[adb] tap 命令启动失败: ({}, {}) - {}", x, y, e);
            format!("点击失败: 无法执行input命令 ({})", e)
        }
    }
}

pub async fn swipe(x1: i32, y1: i32, x2: i32, y2: i32) -> String {
    info!("[adb] swipe: ({}, {}) -> ({}, {})", x1, y1, x2, y2);
    match Command::new(INPUT)
        .arg("swipe")
        .arg(x1.to_string())
        .arg(y1.to_string())
        .arg(x2.to_string())
        .arg(y2.to_string())
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("滑动成功: ({}, {}) -> ({}, {})", x1, y1, x2, y2)
            } else {
                let err = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    format!("退出码: {}", o.status.code().unwrap_or(-1))
                };
                warn!("[adb] swipe 失败: ({}, {})->({},{}) - {}", x1, y1, x2, y2, err);
                format!("滑动失败: {}", err)
            }
        }
        Err(e) => {
            warn!("[adb] swipe 命令启动失败: {}", e);
            format!("滑动失败: 无法执行input命令 ({})", e)
        }
    }
}

pub async fn keyevent(key: &str) -> String {
    let key_code = match key {
        "back" => "4",
        "home" => "3",
        "power" => "26",
        "volume_up" => "24",
        "volume_down" => "25",
        "recents" => "187",
        "menu" => "82",
        "enter" => "66",
        "delete" => "67",
        "tab" => "61",
        "space" => "62",
        "camera" => "27",
        "search" => "84",
        "page_up" => "92",
        "page_down" => "93",
        "escape" => "111",
        _ => key,
    };

    info!("[adb] keyevent: {} ({})", key, key_code);
    match Command::new(INPUT)
        .arg("keyevent")
        .arg(key_code)
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("按键模拟成功: {}", key)
            } else {
                let err = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    format!("退出码: {}", o.status.code().unwrap_or(-1))
                };
                warn!("[adb] keyevent 失败: {} - {}", key, err);
                format!("按键模拟失败 ({}): {}", key, err)
            }
        }
        Err(e) => {
            warn!("[adb] keyevent 命令启动失败: {} - {}", key, e);
            format!("按键模拟失败: 无法执行input命令 ({})", e)
        }
    }
}

pub async fn input_text(text: &str) -> String {
    info!("[adb] input_text: {}", text);
    // 使用 input text 时，因为通过 Command::new().arg() 直接传参（不经shell），
    // 只需转义 Android input 命令本身的特殊字符即可，空格无需转义
    let escaped = text
        .replace("\\", "\\\\")
        .replace("'", "\\'")
        .replace("\"", "\\\"")
        .replace(" ", "%s");

    match Command::new(INPUT)
        .arg("text")
        .arg(&escaped)
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("输入成功: {}", text)
            } else {
                let err = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    format!("退出码: {}", o.status.code().unwrap_or(-1))
                };
                warn!("[adb] input_text 失败: {} - {}", text, err);
                format!("输入失败: {}", err)
            }
        }
        Err(e) => {
            warn!("[adb] input_text 命令启动失败: {} - {}", text, e);
            format!("输入失败: 无法执行input命令 ({})", e)
        }
    }
}

pub async fn screencap(filename: &str) -> String {
    info!("[adb] screencap: {}", filename);
    match Command::new(SCREENCAP)
        .arg("-p")
        .arg(filename)
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() {
                format!("截图成功: {}", filename)
            } else {
                let err = if !stderr.trim().is_empty() {
                    stderr.trim().to_string()
                } else {
                    format!("退出码: {}", o.status.code().unwrap_or(-1))
                };
                warn!("[adb] screencap 失败: {} - {}", filename, err);
                format!("截图失败: {}", err)
            }
        }
        Err(e) => {
            warn!("[adb] screencap 命令启动失败: {} - {}", filename, e);
            format!("截图失败: 无法执行screencap命令 ({})", e)
        }
    }
}

/// 截图并返回 base64 编码（供AI视觉分析）
pub async fn adb_screencap_base64() -> Result<String, String> {
    use base64::Engine;
    info!("[adb] screencap_base64");
    let output = Command::new(SCREENCAP)
        .arg("-p")
        .output()
        .await
        .map_err(|e| format!("截图失败: {}", e))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("截图命令执行失败: {}", stderr.trim()));
    }
    Ok(base64::engine::general_purpose::STANDARD.encode(&output.stdout))
}

pub async fn reboot() -> String {
    info!("[adb] reboot");
    match Command::new(REBOOT).output().await {
        Ok(_) => "设备正在重启...".to_string(),
        Err(e) => format!("重启失败: {}", e),
    }
}

pub async fn shutdown() -> String {
    info!("[adb] shutdown");
    match Command::new(REBOOT).arg("shutdown").output().await {
        Ok(_) => "设备正在关机...".to_string(),
        Err(e) => format!("关机失败: {}", e),
    }
}

pub async fn tts(text: &str) -> String {
    info!("[adb] tts: {}", text);
    // 方法1: 使用 Android TTS Engine 的标准广播
    match Command::new(AM)
        .arg("broadcast")
        .arg("-a")
        .arg("com.android.tts.SPEAK")
        .arg("--es")
        .arg("text")
        .arg(text)
        .arg("--ei")
        .arg("stream")
        .arg("3")
        .output()
        .await
    {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() && !output.contains("Error") {
                return format!("TTS语音播放成功: {}", text);
            }
        }
        Err(_) => {}
    }
    
    // 方法2: 使用 cmd speech 命令触发 TTS（部分设备支持）
    match Command::new(CMD)
        .arg("speech")
        .arg("speak")
        .arg(text)
        .output()
        .await
    {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            let stderr = String::from_utf8_lossy(&o.stderr);
            if o.status.success() && !output.contains("Error") && !output.contains("Unknown") {
                return format!("TTS语音播放成功: {}", text);
            }
        }
        Err(_) => {}
    }
    
    // 方法3: 使用 content provider 触发 TTS（兼容更多设备）
    match Command::new(SH)
        .arg("-c")
        .arg(format!(
            "content call --uri content://com.android.providers.settings/system --method GET_system --arg tts_default_synth --extra _value:s:{} 2>/dev/null || am startservice -a android.intent.action.TTS_SERVICE --es text '{}'",
            text.replace('\'', "'\\''"),
            text.replace('\'', "'\\''")
        ))
        .output()
        .await
    {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout);
            if !output.contains("error") && !output.contains("Error") && !output.contains("Unknown") {
                return format!("TTS语音播放成功: {}", text);
            }
        }
        Err(_) => {}
    }
    
    format!("TTS语音播放失败: 设备不支持TTS命令，请安装TTS引擎")
}

pub async fn tts_speak(text: &str, engine: Option<String>) -> String {
    let escaped_text = text.replace("'", "\\'").replace("\"", "\\\"");
    
    let mut cmd = Command::new(AM);
    cmd.arg("broadcast")
        .arg("-a")
        .arg("com.android.tts.speak")
        .arg("--es")
        .arg("text")
        .arg(&escaped_text);
    
    if let Some(ref engine_name) = engine {
        cmd.arg("--es")
            .arg("engine")
            .arg(engine_name);
    }
    
    match cmd.output().await {
        Ok(o) => {
            if o.status.success() {
                format!("TTS语音播放成功: {}", text)
            } else {
                let stderr = String::from_utf8_lossy(&o.stderr);
                format!("TTS语音播放失败: {}", stderr.trim())
            }
        }
        Err(e) => format!("TTS语音播放失败: {}", e),
    }
}
