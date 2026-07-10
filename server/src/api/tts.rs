use axum::Json;
use serde::Deserialize;
use std::collections::HashSet;
use tokio::process::Command;

use crate::data::response::ApiResponse;

#[derive(Debug, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub engine: Option<String>,
    pub language: Option<String>,
    pub pitch: Option<f32>,
    pub rate: Option<f32>,
    pub volume: Option<f32>,
}

/// 已知的 TTS 引擎包名 → 显示名称映射
fn known_engine_label(pkg: &str) -> Option<&'static str> {
    match pkg {
        "com.google.android.tts" => Some("Google 文字转语音"),
        "com.svox.pico" => Some("Pico TTS"),
        "com.android.tts" => Some("系统默认 TTS"),
        "com.miui.tts" => Some("小米 TTS"),
        "com.xiaomi.misettings" => Some("小米设置 TTS"),
        "com.huawei.tts" => Some("华为 TTS"),
        "com.samsung.android.tts" => Some("三星 TTS"),
        "com.baidu.tts" => Some("百度 TTS"),
        "com.iflytek.tts" => Some("讯飞 TTS"),
        "com.iflytek.speechcloud" => Some("讯飞语音云"),
        "com.baidu.duersdk" => Some("度秘 TTS"),
        "com.cetcnav.tts" => Some("导航 TTS"),
        "com.nuance.tts" => Some("Nuance TTS"),
        "com.ivona.tts" => Some("IVONA TTS"),
        "com.amazon.tts" => Some("Amazon TTS"),
        "com.microsoft.tts" => Some("微软 TTS"),
        "com.cereproc.tts" => Some("CereProc TTS"),
        "com.acapelagroup.tts" => Some("Acapela TTS"),
        _ => None,
    }
}

/// 标签化引擎包名：优先使用已知名称，否则取最后一段作为标签
fn label_for_engine(pkg: &str) -> String {
    if let Some(label) = known_engine_label(pkg) {
        return label.to_string();
    }
    // 取包名最后一段作为可读标签
    let short = pkg.rsplit('.').next().unwrap_or(pkg);
    if short.to_lowercase().contains("tts") {
        format!("{} ({})", short.to_uppercase(), pkg)
    } else {
        format!("{} TTS ({})", short, pkg)
    }
}

/// 获取默认 TTS 引擎包名
async fn get_default_engine() -> Option<String> {
    // 方法1: settings get secure tts_default_synth
    if let Ok(output) = Command::new("/system/bin/settings")
        .args(["get", "secure", "tts_default_synth"])
        .output()
        .await
    {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !val.is_empty() && val != "null" {
            return Some(val);
        }
    }
    // 方法2: settings get system tts_default_synth
    if let Ok(output) = Command::new("/system/bin/settings")
        .args(["get", "system", "tts_default_synth"])
        .output()
        .await
    {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !val.is_empty() && val != "null" {
            return Some(val);
        }
    }
    None
}

/// 从 Android settings 读取一个 f32 值
async fn get_system_setting(key: &str) -> Option<f32> {
    if let Ok(output) = Command::new("/system/bin/settings")
        .args(["get", "system", key])
        .output()
        .await
    {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !val.is_empty() && val != "null" {
            return val.parse::<f32>().ok();
        }
    }
    None
}

/// 通过 `cmd tts list engines` 获取引擎列表
async fn list_engines_cmd() -> Vec<String> {
    let mut engines = Vec::new();
    if let Ok(output) = Command::new("/system/bin/cmd")
        .args(["tts", "list", "engines"])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("Engines:") || trimmed.starts_with('-') {
                continue;
            }
            // cmd tts list engines 每行格式可能是：
            //   "com.google.android.tts"
            //   "com.google.android.tts Google TTS"
            //   "  com.google.android.tts"
            let pkg = trimmed.split_whitespace().next().unwrap_or("");
            // 验证是否是合法包名格式 (至少包含一个'.')
            if pkg.contains('.') && !pkg.starts_with('#') {
                engines.push(pkg.to_string());
            }
        }
    }
    engines
}

/// 通过 `pm list packages` 搜索 TTS 相关包
async fn list_engines_pm() -> Vec<String> {
    let mut engines = Vec::new();
    if let Ok(output) = Command::new("/system/bin/pm")
        .args(["list", "packages"])
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let tts_keywords = [
            "tts", "speech", "pico", "svox",
            "iflytek", "baidu.tts", "huawei.tts",
        ];
        for line in stdout.lines() {
            let lower = line.to_lowercase();
            if tts_keywords.iter().any(|kw| lower.contains(kw)) {
                if let Some(pkg) = line.strip_prefix("package:") {
                    engines.push(pkg.trim().to_string());
                } else if let Some(pos) = line.find('=') {
                    engines.push(line[pos + 1..].trim().to_string());
                }
            }
        }
    }
    engines
}

/// 通过 `dumpsys texttospeech` 获取已注册引擎
async fn list_engines_dumpsys() -> Vec<String> {
    let mut engines = Vec::new();
    if let Ok(output) = Command::new("/system/bin/dumpsys")
        .arg("texttospeech")
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let trimmed = line.trim();
            // 匹配 "Engine: com.xxx.yyy" 或类似格式
            if trimmed.starts_with("Engine:") || trimmed.starts_with("engine:") {
                let pkg = trimmed.split_whitespace().nth(1).unwrap_or("");
                if pkg.contains('.') {
                    engines.push(pkg.to_string());
                }
            }
        }
    }
    engines
}

pub async fn get_tts_engines() -> Json<ApiResponse<Vec<TtsEngineInfo>>> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut engines: Vec<TtsEngineInfo> = Vec::new();

    // 1. 获取默认引擎（优先显示）
    let default_engine = get_default_engine().await;

    // 2. 通过 cmd tts list engines 获取
    for pkg in list_engines_cmd().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo {
                label: label_for_engine(&pkg),
                package_name: pkg,
            });
        }
    }

    // 3. 通过 pm list packages 补充
    for pkg in list_engines_pm().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo {
                label: label_for_engine(&pkg),
                package_name: pkg,
            });
        }
    }

    // 4. 通过 dumpsys texttospeech 补充
    for pkg in list_engines_dumpsys().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo {
                label: label_for_engine(&pkg),
                package_name: pkg,
            });
        }
    }

    // 5. 如果仍然为空，添加常见的默认引擎
    if engines.is_empty() {
        let defaults = [
            "com.google.android.tts",
            "com.android.tts",
            "com.svox.pico",
            "com.miui.tts",
            "com.iflytek.tts",
            "com.baidu.tts",
        ];
        for pkg in defaults {
            if seen.insert(pkg.to_string()) {
                engines.push(TtsEngineInfo {
                    label: label_for_engine(pkg),
                    package_name: pkg.to_string(),
                });
            }
        }
    }

    // 将默认引擎排到第一位
    if let Some(ref default_pkg) = default_engine {
        if let Some(pos) = engines.iter().position(|e| &e.package_name == default_pkg) {
            let item = engines.remove(pos);
            engines.insert(0, item);
        }
        // 标记默认引擎
        if let Some(first) = engines.first_mut() {
            if &first.package_name == default_pkg {
                first.label = format!("[默认] {}", first.label);
            }
        }
    }

    Json(ApiResponse::ok(engines))
}

pub async fn speak(Json(req): Json<TtsRequest>) -> Json<ApiResponse<String>> {
    let text = req.text.trim();
    if text.is_empty() {
        return Json(ApiResponse::err("文本内容不能为空"));
    }

    // 通过 settings 预设 TTS 参数（cmd tts speak 不支持 -r/-p/-v 标志）
    if let Some(rate) = req.rate {
        if rate != 1.0 {
            let _ = Command::new("/system/bin/settings")
                .args(["put", "system", "tts_default_rate", &format!("{:.1}", rate)])
                .output()
                .await;
        }
    }
    if let Some(pitch) = req.pitch {
        if pitch != 1.0 {
            let _ = Command::new("/system/bin/settings")
                .args(["put", "system", "tts_default_pitch", &format!("{:.1}", pitch)])
                .output()
                .await;
        }
    }
    if let Some(volume) = req.volume {
        if volume != 1.0 {
            let _ = Command::new("/system/bin/settings")
                .args(["put", "system", "tts_default_volume", &format!("{:.1}", volume)])
                .output()
                .await;
        }
    }

    // 方法1: 使用 cmd tts speak（标准 Android 方式）
    // 注意: cmd tts speak 只支持 -e (引擎) 和 -l (语言) 标志
    // 不支持 -r/-p/-v 等参数（那些通过 settings 预设）
    let mut cmd = Command::new("/system/bin/cmd");
    cmd.arg("tts").arg("speak");
    if let Some(ref engine) = req.engine {
        cmd.arg("-e").arg(engine);
    }
    if let Some(ref lang) = req.language {
        cmd.arg("-l").arg(lang);
    }
    cmd.arg(text);

    match cmd.output().await {
        Ok(output) => {
            if output.status.success() {
                return Json(ApiResponse::ok_msg("语音播放成功".to_string(), text));
            }
            // cmd tts speak 失败，尝试备用方案
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("[TTS] cmd tts speak failed: {}", stderr);
        }
        Err(e) => {
            eprintln!("[TTS] cmd tts speak error: {}", e);
        }
    }

    // 方法1 失败，返回错误提示
    Json(ApiResponse::err(&format!(
        "TTS 播放失败，请确认设备已安装 TTS 引擎且已在系统设置中配置"
    )))
}

pub async fn stop_tts() -> Json<ApiResponse<String>> {
    // 方法1: cmd tts stop
    let result = Command::new("/system/bin/cmd")
        .args(["tts", "stop"])
        .status()
        .await;

    match result {
        Ok(status) if status.success() => {
            return Json(ApiResponse::ok("语音播放已停止".to_string()));
        }
        _ => {}
    }

    // 方法2: 强制停止所有 TTS 进程
    let engines = ["com.google.android.tts", "com.miui.tts", "com.iflytek.tts"];
    for engine in engines {
        let _ = Command::new("/system/bin/am")
            .args(["force-stop", engine])
            .status()
            .await;
    }

    Json(ApiResponse::ok("语音播放已停止".to_string()))
}

#[derive(Debug, serde::Serialize)]
pub struct TtsEngineInfo {
    pub package_name: String,
    pub label: String,
}

#[derive(Debug, serde::Serialize)]
pub struct TtsSettings {
    pub default_engine: Option<String>,
    pub default_rate: f32,
    pub default_pitch: f32,
    pub default_volume: f32,
    pub available_engines: Vec<TtsEngineInfo>,
}

/// 获取当前TTS设置
pub async fn get_tts_settings() -> Json<ApiResponse<TtsSettings>> {
    let default_engine = get_default_engine().await;
    
    // 从系统 settings 读取当前 TTS 参数
    let rate = get_system_setting("tts_default_rate").await.unwrap_or(1.0);
    let pitch = get_system_setting("tts_default_pitch").await.unwrap_or(1.0);
    let volume = get_system_setting("tts_default_volume").await.unwrap_or(1.0);
    
    // 获取可用引擎列表（使用 HashSet 去重）
    let mut seen: HashSet<String> = HashSet::new();
    let mut engines: Vec<TtsEngineInfo> = Vec::new();
    for pkg in list_engines_cmd().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo {
                label: label_for_engine(&pkg),
                package_name: pkg,
            });
        }
    }
    for pkg in list_engines_pm().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo {
                label: label_for_engine(&pkg),
                package_name: pkg,
            });
        }
    }
    
    Json(ApiResponse::ok(TtsSettings {
        default_engine,
        default_rate: rate,
        default_pitch: pitch,
        default_volume: volume,
        available_engines: engines,
    }))
}

/// 设置TTS参数
#[derive(Debug, Deserialize)]
pub struct TtsSettingsRequest {
    pub rate: Option<f32>,
    pub pitch: Option<f32>,
    pub volume: Option<f32>,
}

pub async fn update_tts_settings(Json(req): Json<TtsSettingsRequest>) -> Json<ApiResponse<String>> {
    if let Some(rate) = req.rate {
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_rate", &format!("{:.1}", rate)])
            .output()
            .await;
    }
    
    if let Some(pitch) = req.pitch {
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_pitch", &format!("{:.1}", pitch)])
            .output()
            .await;
    }

    if let Some(volume) = req.volume {
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_volume", &format!("{:.1}", volume)])
            .output()
            .await;
    }
    
    Json(ApiResponse::ok("TTS设置已更新".to_string()))
}

/// 设置默认TTS引擎
#[derive(Debug, Deserialize)]
pub struct SetDefaultEngineRequest {
    pub engine: String,
}

pub async fn set_default_engine(Json(req): Json<SetDefaultEngineRequest>) -> Json<ApiResponse<String>> {
    if req.engine.is_empty() {
        return Json(ApiResponse::err("引擎包名不能为空"));
    }
    
    // 设置默认引擎
    let _ = Command::new("/system/bin/settings")
        .args(["put", "secure", "tts_default_synth", &req.engine])
        .output()
        .await;
    
    Json(ApiResponse::ok(format!("已设置默认TTS引擎为: {}", req.engine)))
}

/// 测试TTS功能
#[derive(Debug, Deserialize)]
pub struct TtsTestRequest {
    pub text: Option<String>,
    pub engine: Option<String>,
}

pub async fn test_tts(Json(req): Json<TtsTestRequest>) -> Json<ApiResponse<String>> {
    let text = req.text.unwrap_or_else(|| "这是一段测试语音，TaskMod TTS功能正常。".to_string());
    let engine = req.engine.unwrap_or_else(|| "com.google.android.tts".to_string());
    
    // 调用speak功能
    let tts_req = TtsRequest {
        text,
        engine: Some(engine),
        language: None,
        pitch: Some(1.0),
        rate: Some(1.0),
        volume: Some(1.0),
    };
    
    speak(Json(tts_req)).await
}