use axum::Json;
use serde::Deserialize;
use std::collections::HashSet;
use tokio::process::Command;

use crate::data::response::ApiResponse;
use crate::data::tts_config::{EngineParams, ReplaceRule, TtsConfig};

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

fn label_for_engine(pkg: &str) -> String {
    if let Some(label) = known_engine_label(pkg) {
        return label.to_string();
    }
    let short = pkg.rsplit('.').next().unwrap_or(pkg);
    if short.to_lowercase().contains("tts") {
        format!("{} ({})", short.to_uppercase(), pkg)
    } else {
        format!("{} TTS ({})", short, pkg)
    }
}

async fn get_default_engine() -> Option<String> {
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
            let pkg = trimmed.split_whitespace().next().unwrap_or("");
            if pkg.contains('.') && !pkg.starts_with('#') {
                engines.push(pkg.to_string());
            }
        }
    }
    engines
}

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

async fn discover_engines() -> Vec<TtsEngineInfo> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut engines: Vec<TtsEngineInfo> = Vec::new();
    let default_engine = get_default_engine().await;

    for pkg in list_engines_cmd().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo { label: label_for_engine(&pkg), package_name: pkg });
        }
    }
    for pkg in list_engines_pm().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo { label: label_for_engine(&pkg), package_name: pkg });
        }
    }
    for pkg in list_engines_dumpsys().await {
        if seen.insert(pkg.clone()) {
            engines.push(TtsEngineInfo { label: label_for_engine(&pkg), package_name: pkg });
        }
    }

    if engines.is_empty() {
        let defaults = [
            "com.google.android.tts", "com.android.tts", "com.svox.pico",
            "com.miui.tts", "com.iflytek.tts", "com.baidu.tts",
        ];
        for pkg in defaults {
            if seen.insert(pkg.to_string()) {
                engines.push(TtsEngineInfo { label: label_for_engine(pkg), package_name: pkg.to_string() });
            }
        }
    }

    if let Some(ref default_pkg) = default_engine {
        if let Some(pos) = engines.iter().position(|e| &e.package_name == default_pkg) {
            let item = engines.remove(pos);
            engines.insert(0, item);
        }
        if let Some(first) = engines.first_mut() {
            if &first.package_name == default_pkg {
                first.label = format!("[默认] {}", first.label);
            }
        }
    }
    engines
}

/// 通过 Android settings 预设 TTS 参数
async fn apply_tts_params(rate: f32, pitch: f32, volume: f32) {
    if rate != 1.0 {
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_rate", &format!("{:.1}", rate)])
            .output().await;
    }
    if pitch != 1.0 {
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_pitch", &format!("{:.1}", pitch)])
            .output().await;
    }
    if volume != 1.0 {
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_volume", &format!("{:.1}", volume)])
            .output().await;
    }
}

/// 执行单次 cmd tts speak
async fn exec_speak(text: &str, engine: Option<&str>, language: Option<&str>) -> bool {
    let mut cmd = Command::new("/system/bin/cmd");
    cmd.arg("tts").arg("speak");
    if let Some(e) = engine {
        cmd.arg("-e").arg(e);
    }
    if let Some(l) = language {
        cmd.arg("-l").arg(l);
    }
    cmd.arg(text);
    match cmd.output().await {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

// ==================== API Handlers ====================

pub async fn get_tts_engines() -> Json<ApiResponse<Vec<TtsEngineInfo>>> {
    Json(ApiResponse::ok(discover_engines().await))
}

/// 朗读文本（集成替换规则 + 分句 + 按引擎参数）
pub async fn speak(Json(req): Json<TtsRequest>) -> Json<ApiResponse<String>> {
    let text = req.text.trim();
    if text.is_empty() {
        return Json(ApiResponse::err("文本内容不能为空"));
    }

    let config = TtsConfig::load().await;

    // 1. 应用文本替换规则
    let processed_text = config.apply_replace_rules(text);

    // 2. 获取引擎参数（请求参数 > 配置中按引擎参数 > 全局参数）
    let engine = req.engine.as_deref().unwrap_or("");
    let (cfg_rate, cfg_pitch, cfg_volume) = if !engine.is_empty() {
        config.get_engine_params(engine)
    } else {
        (config.global_rate, config.global_pitch, config.global_volume)
    };
    let rate = req.rate.unwrap_or(cfg_rate);
    let pitch = req.pitch.unwrap_or(cfg_pitch);
    let volume = req.volume.unwrap_or(cfg_volume);

    // 3. 预设 TTS 参数到系统 settings
    apply_tts_params(rate, pitch, volume).await;

    // 4. 分句并逐句朗读
    let sentences = config.split_sentences(&processed_text);
    let total = sentences.len();
    let mut failed = 0;

    for (i, sentence) in sentences.iter().enumerate() {
        let ok = exec_speak(sentence, req.engine.as_deref(), req.language.as_deref()).await;
        if !ok {
            eprintln!("[TTS] 第 {}/{} 句朗读失败: {}", i + 1, total, sentence);
            failed += 1;
        }
        // 多句之间暂停 200ms，避免叠音
        if i < total - 1 {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    }

    if failed == 0 {
        Json(ApiResponse::ok_msg("语音播放成功".to_string(), text))
    } else if failed < total {
        Json(ApiResponse::ok_msg(
            format!("部分朗读成功 ({}/{})", total - failed, total),
            text,
        ))
    } else {
        Json(ApiResponse::err(
            "TTS 播放失败，请确认设备已安装 TTS 引擎且已在系统设置中配置",
        ))
    }
}

pub async fn stop_tts() -> Json<ApiResponse<String>> {
    let result = Command::new("/system/bin/cmd")
        .args(["tts", "stop"])
        .status().await;
    if let Ok(status) = result {
        if status.success() {
            return Json(ApiResponse::ok("语音播放已停止".to_string()));
        }
    }
    for engine in ["com.google.android.tts", "com.miui.tts", "com.iflytek.tts"] {
        let _ = Command::new("/system/bin/am")
            .args(["force-stop", engine])
            .status().await;
    }
    Json(ApiResponse::ok("语音播放已停止".to_string()))
}

/// 获取 TTS 设置（合并系统设置 + 本地配置）
pub async fn get_tts_settings() -> Json<ApiResponse<TtsSettings>> {
    let config = TtsConfig::load().await;
    let default_engine = get_default_engine().await
        .unwrap_or_else(|| config.default_engine.clone());
    let engines = discover_engines().await;

    Json(ApiResponse::ok(TtsSettings {
        default_engine: if default_engine.is_empty() { None } else { Some(default_engine) },
        default_rate: config.global_rate,
        default_pitch: config.global_pitch,
        default_volume: config.global_volume,
        replace_enabled: config.replace_enabled,
        split_enabled: config.split_enabled,
        engine_params: config.engine_params,
        available_engines: engines,
    }))
}

/// 更新全局 TTS 设置
#[derive(Debug, Deserialize)]
pub struct TtsSettingsRequest {
    pub rate: Option<f32>,
    pub pitch: Option<f32>,
    pub volume: Option<f32>,
    pub replace_enabled: Option<bool>,
    pub split_enabled: Option<bool>,
}

pub async fn update_tts_settings(Json(req): Json<TtsSettingsRequest>) -> Json<ApiResponse<String>> {
    let mut config = TtsConfig::load().await;

    if let Some(rate) = req.rate {
        config.global_rate = rate;
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_rate", &format!("{:.1}", rate)])
            .output().await;
    }
    if let Some(pitch) = req.pitch {
        config.global_pitch = pitch;
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_pitch", &format!("{:.1}", pitch)])
            .output().await;
    }
    if let Some(volume) = req.volume {
        config.global_volume = volume;
        let _ = Command::new("/system/bin/settings")
            .args(["put", "system", "tts_default_volume", &format!("{:.1}", volume)])
            .output().await;
    }
    if let Some(enabled) = req.replace_enabled {
        config.replace_enabled = enabled;
    }
    if let Some(enabled) = req.split_enabled {
        config.split_enabled = enabled;
    }

    match config.save().await {
        Ok(_) => Json(ApiResponse::ok("TTS设置已保存".to_string())),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 设置默认引擎
#[derive(Debug, Deserialize)]
pub struct SetDefaultEngineRequest {
    pub engine: String,
}

pub async fn set_default_engine(Json(req): Json<SetDefaultEngineRequest>) -> Json<ApiResponse<String>> {
    if req.engine.is_empty() {
        return Json(ApiResponse::err("引擎包名不能为空"));
    }
    let _ = Command::new("/system/bin/settings")
        .args(["put", "secure", "tts_default_synth", &req.engine])
        .output().await;
    // 同步到本地配置
    let mut config = TtsConfig::load().await;
    config.default_engine = req.engine.clone();
    let _ = config.save().await;
    Json(ApiResponse::ok(format!("已设置默认TTS引擎为: {}", req.engine)))
}

/// 测试/试听 TTS
#[derive(Debug, Deserialize)]
pub struct TtsTestRequest {
    pub text: Option<String>,
    pub engine: Option<String>,
}

pub async fn test_tts(Json(req): Json<TtsTestRequest>) -> Json<ApiResponse<String>> {
    let text = req.text.unwrap_or_else(|| "这是一段测试语音，TaskMod TTS功能正常。".to_string());
    let tts_req = TtsRequest {
        text,
        engine: req.engine,
        language: None,
        pitch: None,
        rate: None,
        volume: None,
    };
    speak(Json(tts_req)).await
}

// ==================== 按引擎参数 CRUD ====================

/// 获取所有引擎参数列表
pub async fn get_engine_params() -> Json<ApiResponse<Vec<EngineParams>>> {
    let config = TtsConfig::load().await;
    Json(ApiResponse::ok(config.engine_params))
}

/// 添加/更新引擎参数
pub async fn upsert_engine_params(Json(req): Json<EngineParams>) -> Json<ApiResponse<String>> {
    let mut config = TtsConfig::load().await;
    if let Some(existing) = config.engine_params.iter_mut().find(|p| p.engine == req.engine) {
        existing.rate = req.rate;
        existing.pitch = req.pitch;
        existing.volume = req.volume;
    } else {
        config.engine_params.push(req);
    }
    match config.save().await {
        Ok(_) => Json(ApiResponse::ok("引擎参数已保存".to_string())),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除引擎参数
pub async fn delete_engine_params(axum::extract::Path(engine): axum::extract::Path<String>) -> Json<ApiResponse<String>> {
    let mut config = TtsConfig::load().await;
    config.engine_params.retain(|p| p.engine != engine);
    match config.save().await {
        Ok(_) => Json(ApiResponse::ok("引擎参数已删除".to_string())),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

// ==================== 替换规则 CRUD ====================

/// 获取所有替换规则
pub async fn get_replace_rules() -> Json<ApiResponse<Vec<ReplaceRule>>> {
    let config = TtsConfig::load().await;
    Json(ApiResponse::ok(config.replace_rules))
}

/// 添加替换规则
pub async fn add_replace_rule(Json(req): Json<ReplaceRule>) -> Json<ApiResponse<String>> {
    let mut config = TtsConfig::load().await;
    config.replace_rules.push(req);
    match config.save().await {
        Ok(_) => Json(ApiResponse::ok("替换规则已添加".to_string())),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 更新替换规则
pub async fn update_replace_rule(
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<ReplaceRule>,
) -> Json<ApiResponse<String>> {
    let mut config = TtsConfig::load().await;
    if let Some(existing) = config.replace_rules.iter_mut().find(|r| r.id == id) {
        *existing = req;
    } else {
        return Json(ApiResponse::err("规则不存在"));
    }
    match config.save().await {
        Ok(_) => Json(ApiResponse::ok("替换规则已更新".to_string())),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除替换规则
pub async fn delete_replace_rule(axum::extract::Path(id): axum::extract::Path<String>) -> Json<ApiResponse<String>> {
    let mut config = TtsConfig::load().await;
    config.replace_rules.retain(|r| r.id != id);
    match config.save().await {
        Ok(_) => Json(ApiResponse::ok("替换规则已删除".to_string())),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 批量更新替换规则顺序
#[derive(Debug, Deserialize)]
pub struct ReorderRequest {
    pub ids: Vec<String>,
}

pub async fn reorder_replace_rules(Json(req): Json<ReorderRequest>) -> Json<ApiResponse<String>> {
    let mut config = TtsConfig::load().await;
    for (i, id) in req.ids.iter().enumerate() {
        if let Some(rule) = config.replace_rules.iter_mut().find(|r| &r.id == id) {
            rule.order = i as i32;
        }
    }
    match config.save().await {
        Ok(_) => Json(ApiResponse::ok("规则顺序已更新".to_string())),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

// ==================== 响应结构体 ====================

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
    pub replace_enabled: bool,
    pub split_enabled: bool,
    pub engine_params: Vec<EngineParams>,
    pub available_engines: Vec<TtsEngineInfo>,
}
