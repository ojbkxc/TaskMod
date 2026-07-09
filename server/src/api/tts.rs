use axum::{Json};
use serde::Deserialize;
use tokio::process::Command;

use crate::data::response::ApiResponse;
use crate::utils::adb;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct TtsRequest {
    pub text: String,
    pub engine: Option<String>,
    pub language: Option<String>,
    pub pitch: Option<f32>,
    pub rate: Option<f32>,
    pub volume: Option<f32>,
}

pub async fn get_tts_engines() -> Json<ApiResponse<Vec<TtsEngineInfo>>> {
    let output = match Command::new("/system/bin/pm")
        .arg("list")
        .arg("packages")
        .arg("-f")
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => return Json(ApiResponse::err(&format!("执行命令失败: {}", e))),
    };
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut engines = Vec::new();
    
    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if lower.contains("tts") || lower.contains("speech") || lower.contains("voice")
            || lower.contains("speak") || lower.contains("pico")
            || lower.contains("svox") || lower.contains("ivona")
            || lower.contains("xiaomi") && lower.contains("ai")
            || lower.contains("miui.tts") || lower.contains("google.tts")
        {
            let parts: Vec<&str> = line.split('=').collect();
            if parts.len() >= 2 {
                let package_name = parts[1].trim();
                let label = package_name.replace(".", " ").replace("tts", "TTS");
                engines.push(TtsEngineInfo {
                    package_name: package_name.to_string(),
                    label: format!("{} ({})", label, package_name),
                });
            }
        }
    }
    
    if engines.is_empty() {
        engines.push(TtsEngineInfo {
            package_name: "com.android.tts".to_string(),
            label: "默认TTS引擎".to_string(),
        });
    }
    
    Json(ApiResponse::ok(engines))
}

pub async fn speak(Json(req): Json<TtsRequest>) -> Json<ApiResponse<String>> {
    let text = req.text.trim();
    if text.is_empty() {
        return Json(ApiResponse::err("文本内容不能为空"));
    }

    // Try cmd tts speak first (Android 10+)
    let mut cmd = Command::new("/system/bin/cmd");
    cmd.arg("tts").arg("speak").arg(text);

    if let Some(ref engine) = req.engine {
        cmd.arg("-e").arg(engine);
    }
    if let Some(ref lang) = req.language {
        cmd.arg("-l").arg(lang);
    }

    match cmd.output().await {
        Ok(output) => {
            if output.status.success() {
                return Json(ApiResponse::ok_msg("语音播放成功".to_string(), text));
            }
            // If cmd tts failed, try am broadcast as fallback
        }
        Err(_) => {}
    }

    // Fallback: am broadcast
    let escaped_text = text.replace("'", "\\'").replace("\"", "\\\"");
    let mut cmd_parts: Vec<String> = vec!["/system/bin/am", "broadcast",
        "-a", "com.google.android.tts.engine.TTS_SPEAK",
        "--es", "text", &escaped_text]
        .into_iter().map(String::from).collect();

    if let Some(ref engine) = req.engine {
        cmd_parts.extend(vec!["--es", "engine", engine.as_str()].into_iter().map(String::from));
    }

    match adb::execute_command(&cmd_parts).await {
        Ok(output) => {
            if output.status.success() {
                Json(ApiResponse::ok_msg("语音播放成功".to_string(), text))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Json(ApiResponse::err(&format!("播放失败: {}", stderr)))
            }
        }
        Err(e) => Json(ApiResponse::err(&format!("执行命令失败: {}", e))),
    }
}

#[derive(Debug, serde::Serialize)]
pub struct TtsEngineInfo {
    pub package_name: String,
    pub label: String,
}