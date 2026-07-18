use serde::Serialize;

/// 跨平台设备信息
#[derive(Debug, Serialize)]
pub struct DeviceInfo {
    pub model: String,
    pub os_version: String,
    pub screen_size: String,
    pub battery: String,
    pub ip: String,
    pub storage: String,
    pub cpu: String,
    pub memory: String,
    pub wifi: String,
    pub platform: String,
}

/// 获取设备信息（跨平台）
pub async fn get_device_info() -> DeviceInfo {
    let platform = crate::platform::detect_platform();

    match platform {
        crate::platform::Platform::Android => get_android_info().await,
        crate::platform::Platform::Windows => get_windows_info().await,
        crate::platform::Platform::Linux => get_linux_info().await,
        _ => DeviceInfo {
            model: "Unknown".to_string(),
            os_version: "Unknown".to_string(),
            screen_size: "Unknown".to_string(),
            battery: "Unknown".to_string(),
            ip: "Unknown".to_string(),
            storage: "Unknown".to_string(),
            cpu: "Unknown".to_string(),
            memory: "Unknown".to_string(),
            wifi: "Unknown".to_string(),
            platform: format!("{:?}", platform),
        },
    }
}

// ==================== Android ====================

async fn get_android_info() -> DeviceInfo {
    let (model, os_version, ip, wifi) = tokio::join!(
        get_prop("ro.product.model"),
        get_prop("ro.build.version.release"),
        get_ip_address(),
        get_wifi_ssid(),
    );

    let screen_size = run_cmd_output("wm", &["size"])
        .await
        .and_then(|s| s.lines().last().map(|l| l.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let battery = run_cmd_output("dumpsys", &["battery"])
        .await
        .and_then(|s| {
            s.lines()
                .find(|l| l.contains("level"))
                .map(|l| l.trim().to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let storage = run_cmd_output("df", &["-h", "/data"])
        .await
        .and_then(|s| s.lines().nth(1).map(|l| l.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let cpu = run_cmd_output("cat", &["/proc/cpuinfo"])
        .await
        .and_then(|s| {
            s.lines()
                .find(|l| l.contains("Hardware"))
                .map(|l| l.to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let memory = run_cmd_output("cat", &["/proc/meminfo"])
        .await
        .and_then(|s| {
            s.lines()
                .next()
                .map(|l| l.to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    DeviceInfo {
        model,
        os_version,
        screen_size,
        battery,
        ip,
        storage,
        cpu,
        memory,
        wifi,
        platform: "Android".to_string(),
    }
}

// ==================== Windows ====================

async fn get_windows_info() -> DeviceInfo {
    let (model, os_version, ip, cpu, memory) = tokio::join!(
        async { run_ps("(Get-CimInstance Win32_ComputerSystem).Model").await.unwrap_or_else(|| "Unknown".to_string()) },
        async { run_ps("(Get-CimInstance Win32_OperatingSystem).Version").await.unwrap_or_else(|| "Unknown".to_string()) },
        async { run_ps("(Get-NetIPAddress -AddressFamily IPv4 | Where-Object {$_.InterfaceAlias -notlike '*Loopback*'} | Select-Object -First 1).IPAddress").await.unwrap_or_else(|| "Unknown".to_string()) },
        async { run_ps("(Get-CimInstance Win32_Processor).Name").await.unwrap_or_else(|| "Unknown".to_string()) },
        async { run_ps("[math]::Round((Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory / 1GB, 1).ToString() + ' GB'").await.unwrap_or_else(|| "Unknown".to_string()) },
    );

    let screen_size = run_ps("(Get-CimInstance Win32_VideoController).VideoModeDescription")
        .await
        .unwrap_or_else(|| "Unknown".to_string());

    let battery = run_ps("(Get-CimInstance Win32_Battery).EstimatedChargeRemaining")
        .await
        .map(|s| format!("{}%", s))
        .unwrap_or_else(|| "Desktop".to_string());

    let storage = run_ps("Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='C:'\" | ForEach-Object { [math]::Round($_.FreeSpace/1GB,1).ToString() + ' GB free / ' + [math]::Round($_.Size/1GB,1).ToString() + ' GB' }")
        .await
        .unwrap_or_else(|| "Unknown".to_string());

    let wifi = run_ps("(Get-NetConnectionProfile | Where-Object {$_.InterfaceAlias -like '*Wi-Fi*' -or $_.InterfaceAlias -like '*WLAN*'}).Name")
        .await
        .unwrap_or_else(|| "Unknown".to_string());

    DeviceInfo {
        model,
        os_version,
        screen_size,
        battery,
        ip,
        storage,
        cpu,
        memory,
        wifi,
        platform: "Windows".to_string(),
    }
}

// ==================== Linux ====================

async fn get_linux_info() -> DeviceInfo {
    let (model, os_version, ip, cpu, memory) = tokio::join!(
        async {
            run_cmd_output("cat", &["/sys/devices/virtual/dmi/id/product_name"]).await
                .unwrap_or_else(|| "Unknown".to_string())
        },
        async {
            run_cmd_output("cat", &["/etc/os-release"]).await
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.starts_with("PRETTY_NAME="))
                        .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
                })
                .unwrap_or_else(|| "Unknown".to_string())
        },
        async {
            run_cmd_output("hostname", &["-I"]).await
                .and_then(|s| s.split_whitespace().next().map(|s| s.to_string()))
                .unwrap_or_else(|| "Unknown".to_string())
        },
        async {
            run_cmd_output("lscpu", &[]).await
                .and_then(|s| {
                    s.lines()
                        .find(|l| l.contains("Model name"))
                        .and_then(|l| l.split(':').nth(1).map(|s| s.trim().to_string()))
                })
                .unwrap_or_else(|| "Unknown".to_string())
        },
        async {
            run_cmd_output("free", &["-h"]).await
                .and_then(|s| s.lines().nth(1).map(|l| l.to_string()))
                .unwrap_or_else(|| "Unknown".to_string())
        },
    );

    let screen_size = run_cmd_output("xrandr", &[])
        .await
        .and_then(|s| {
            s.lines()
                .find(|l| l.contains(" connected"))
                .map(|l| l.to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let storage = run_cmd_output("df", &["-h", "/"])
        .await
        .and_then(|s| s.lines().nth(1).map(|l| l.to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let wifi = run_cmd_output("iwgetid", &["-r"])
        .await
        .unwrap_or_else(|| "Unknown".to_string());

    DeviceInfo {
        model,
        os_version,
        screen_size,
        battery: "Unknown".to_string(),
        ip,
        storage,
        cpu,
        memory,
        wifi,
        platform: "Linux".to_string(),
    }
}

// ==================== 工具函数 ====================

async fn get_prop(key: &str) -> String {
    run_cmd_output("getprop", &[key]).await.unwrap_or_else(|| "Unknown".to_string())
}

async fn get_ip_address() -> String {
    run_cmd_output("ip", &["route", "get", "1.1.1.1"])
        .await
        .and_then(|s| {
            s.split_whitespace()
                .find(|w| w.chars().all(|c| c.is_ascii_digit() || c == '.'))
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

async fn get_wifi_ssid() -> String {
    run_cmd_output("dumpsys", &["wifi"])
        .await
        .and_then(|s| {
            s.lines()
                .find(|l| l.contains("mWifiInfo"))
                .and_then(|l| {
                    l.split("SSID:").nth(1).map(|s| {
                        s.split(',').next().unwrap_or("").trim().trim_matches('"').to_string()
                    })
                })
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

async fn run_cmd_output(cmd: &str, args: &[&str]) -> Option<String> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .ok()?;

    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
async fn run_ps(script: &str) -> Option<String> {
    let output = tokio::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .await
        .ok()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if result.is_empty() { None } else { Some(result) }
    } else {
        None
    }
}

#[cfg(not(target_os = "windows"))]
async fn run_ps(_script: &str) -> Option<String> {
    None
}
