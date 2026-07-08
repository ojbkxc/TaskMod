use axum::{Json};
use serde::Deserialize;
use std::process::Command;

use crate::data::response::ApiResponse;
use crate::utils::adb;

#[derive(Debug, Deserialize)]
pub struct TtsRequest {
    pub text: String,
    pub engine: Option<String>,
    pub language: Option<String>,
    pub pitch: Option<f32>,
    pub rate: Option<f32>,
    pub volume: Option<f32>,
}

pub async fn get_tts_engines() -> Json<ApiResponse<Vec<TtsEngineInfo>>> {
    let output = match Command::new("adb")
        .arg("shell")
        .arg("pm")
        .arg("list")
        .arg("packages")
        .arg("-f")
        .output()
        .await
    {
        Ok(o) => o,
        Err(e) => return Json(ApiResponse::err(&format!("执行ADB命令失败: {}", e))),
    };
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut engines = Vec::new();
    
    for line in stdout.lines() {
        if line.contains("tts") || line.contains("TTS") || line.contains("speech") {
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
    
    let escaped_text = text.replace("'", "\\'").replace("\"", "\\\"");
    
    let mut cmd_parts = vec!["shell", "am", "broadcast", "-a", "com.android.tts.speak"];
    
    if let Some(ref engine) = req.engine {
        cmd_parts.extend(vec!["--es", "engine", engine]);
    }
    
    cmd_parts.extend(vec!["--es", "text", &escaped_text]);
    
    if let Some(ref lang) = req.language {
        cmd_parts.extend(vec!["--es", "language", lang]);
    }
    
    let output = match adb::execute_command(&cmd_parts).await {
        Ok(o) => o,
        Err(e) => return Json(ApiResponse::err(&format!("执行ADB命令失败: {}", e))),
    };
    
    if output.status.success() {
        Json(ApiResponse::ok_msg("语音播放成功".to_string(), text))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Json(ApiResponse::err(&format!("播放失败: {}", stderr)))
    }
}

#[derive(Debug, serde::Serialize)]
pub struct TtsEngineInfo {
    pub package_name: String,
    pub label: String,
}