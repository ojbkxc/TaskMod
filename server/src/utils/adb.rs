use tokio::process::Command;

pub async fn run_command(cmd: &str) -> Result<String, String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("命令为空".to_string());
    }

    let mut command = Command::new(parts[0]);
    for part in parts.iter().skip(1) {
        command.arg(part);
    }

    match command.output().await {
        Ok(o) => Ok(format!(
            "stdout: {}\nstderr: {}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        )),
        Err(e) => Err(format!("命令执行失败: {}", e)),
    }
}

pub async fn run_command_raw(cmd: &str) -> Result<std::process::Output, String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Err("命令为空".to_string());
    }

    let mut command = Command::new(parts[0]);
    for part in parts.iter().skip(1) {
        command.arg(part);
    }

    command.output().await.map_err(|e| format!("命令执行失败: {}", e))
}

pub async fn get_screen_size() -> String {
    match Command::new("wm").arg("size").output().await {
        Ok(o) => {
            let output = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if output.starts_with("Physical size: ") {
                output.replace("Physical size: ", "")
            } else {
                output
            }
        }
        Err(_) => "unknown".to_string(),
    }
}

pub async fn get_wifi_info() -> String {
    match Command::new("dumpsys").arg("wifi").output().await {
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
    match Command::new("dumpsys").arg("battery").output().await {
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

    if let Ok(o) = Command::new("getprop").arg("ro.product.model").output().await {
        info.push_str(&format!("设备型号: {}\n", String::from_utf8_lossy(&o.stdout).trim()));
    }

    if let Ok(o) = Command::new("getprop").arg("ro.build.version.release").output().await {
        info.push_str(&format!("Android版本: {}\n", String::from_utf8_lossy(&o.stdout).trim()));
    }

    if let Ok(o) = Command::new("df").arg("-h").output().await {
        info.push_str(&format!("存储信息:\n{}\n", String::from_utf8_lossy(&o.stdout)));
    }

    if info.is_empty() {
        "获取设备信息失败".to_string()
    } else {
        info
    }
}

pub async fn get_running_apps() -> String {
    match Command::new("ps").arg("-A").output().await {
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
    match Command::new("am")
        .arg("start")
        .arg("-n")
        .arg(format!("{}/.MainActivity", package_name))
        .output()
        .await
    {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.contains("Error") || stdout.contains("error") {
                match Command::new("monkey")
                    .arg("-p")
                    .arg(package_name)
                    .arg("-c")
                    .arg("android.intent.category.LAUNCHER")
                    .arg("1")
                    .output()
                    .await
                {
                    Ok(o2) => format!("应用启动成功:\n{}", String::from_utf8_lossy(&o2.stdout)),
                    Err(e) => format!("应用启动失败: {}", e),
                }
            } else {
                format!("应用启动成功:\n{}", stdout)
            }
        }
        Err(e) => format!("应用启动失败: {}", e),
    }
}

pub async fn stop_app(package_name: &str) -> String {
    match Command::new("am")
        .arg("force-stop")
        .arg(package_name)
        .output()
        .await
    {
        Ok(o) => format!(
            "应用已停止: {}\nstdout: {}\nstderr: {}",
            package_name,
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => format!("停止应用失败: {}", e),
    }
}

pub async fn clear_app_data(package_name: &str) -> String {
    match Command::new("pm")
        .arg("clear")
        .arg(package_name)
        .output()
        .await
    {
        Ok(o) => format!("数据清除成功:\n{}", String::from_utf8_lossy(&o.stdout)),
        Err(e) => format!("清除数据失败: {}", e),
    }
}

pub async fn tap(x: i32, y: i32) -> String {
    match Command::new("input")
        .arg("tap")
        .arg(x.to_string())
        .arg(y.to_string())
        .output()
        .await
    {
        Ok(_) => format!("点击成功: ({}, {})", x, y),
        Err(e) => format!("点击失败: {}", e),
    }
}

pub async fn swipe(x1: i32, y1: i32, x2: i32, y2: i32) -> String {
    match Command::new("input")
        .arg("swipe")
        .arg(x1.to_string())
        .arg(y1.to_string())
        .arg(x2.to_string())
        .arg(y2.to_string())
        .output()
        .await
    {
        Ok(_) => format!("滑动成功: ({}, {}) -> ({}, {})", x1, y1, x2, y2),
        Err(e) => format!("滑动失败: {}", e),
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
        _ => key,
    };

    match Command::new("input")
        .arg("keyevent")
        .arg(key_code)
        .output()
        .await
    {
        Ok(_) => format!("按键模拟成功: {}", key),
        Err(e) => format!("按键模拟失败: {}", e),
    }
}

pub async fn input_text(text: &str) -> String {
    match Command::new("input")
        .arg("text")
        .arg(text)
        .output()
        .await
    {
        Ok(_) => format!("输入成功: {}", text),
        Err(e) => format!("输入失败: {}", e),
    }
}

pub async fn screencap(filename: &str) -> String {
    match Command::new("screencap")
        .arg("-p")
        .arg(filename)
        .output()
        .await
    {
        Ok(_) => format!("截图成功: {}", filename),
        Err(e) => format!("截图失败: {}", e),
    }
}

pub async fn reboot() -> String {
    match Command::new("reboot").output().await {
        Ok(_) => "设备正在重启...".to_string(),
        Err(e) => format!("重启失败: {}", e),
    }
}

pub async fn shutdown() -> String {
    match Command::new("reboot").arg("shutdown").output().await {
        Ok(_) => "设备正在关机...".to_string(),
        Err(e) => format!("关机失败: {}", e),
    }
}

pub async fn tts(text: &str) -> String {
    match Command::new("am")
        .arg("broadcast")
        .arg("-a")
        .arg("com.android.tts.speak")
        .arg("--es")
        .arg("utterance")
        .arg(text)
        .output()
        .await
    {
        Ok(_) => format!("TTS语音播放成功: {}", text),
        Err(_) => {
            match Command::new("tts")
                .arg("speak")
                .arg(text)
                .output()
                .await
            {
                Ok(_) => format!("TTS语音播放成功: {}", text),
                Err(e) => format!("TTS语音播放失败: {}", e),
            }
        }
    }
}

pub async fn tts_speak(text: &str, engine: Option<String>) -> String {
    let escaped_text = text.replace("'", "\\'").replace("\"", "\\\"");
    
    let mut cmd = Command::new("am");
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
        Ok(_) => format!("TTS语音播放成功: {}", text),
        Err(e) => format!("TTS语音播放失败: {}", e),
    }
}