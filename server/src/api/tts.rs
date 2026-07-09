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
    let mut engines = Vec::new();
    
    let cmd_result = Command::new("/system/bin/cmd")
        .arg("tts")
        .arg("list")
        .arg("engines")
        .output()
        .await;
    
    match cmd_result {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }
                
                let package_name = parts[0];
                let label = if parts.len() > 1 {
                    parts[1..].join(" ").to_string()
                } else {
                    format!("TTS Engine ({})", package_name)
                };
                
                engines.push(TtsEngineInfo {
                    package_name: package_name.to_string(),
                    label,
                });
            }
        }
        Err(e) => {
            eprintln!("[TTS] cmd tts list engines failed: {}", e);
        }
    }
    
    if engines.is_empty() {
        let pm_result = Command::new("/system/bin/pm")
            .arg("list")
            .arg("packages")
            .output()
            .await;
        
        if let Ok(output) = pm_result {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let tts_keywords = ["tts", "speech", "voice", "pico", "svox", "google.android.tts", "com.google.android.tts", "miui.tts"];
            
            for line in stdout.lines() {
                let lower = line.to_lowercase();
                if tts_keywords.iter().any(|kw| lower.contains(kw)) {
                    let parts: Vec<&str> = line.split('=').collect();
                    if parts.len() >= 2 {
                        let package_name = parts[1].trim();
                        let label = format!("{}-TTS", package_name.split('.').last().unwrap_or(package_name));
                        engines.push(TtsEngineInfo {
                            package_name: package_name.to_string(),
                            label,
                        });
                    }
                }
            }
        }
    }
    
    if engines.is_empty() {
        engines.push(TtsEngineInfo {
            package_name: "com.android.tts".to_string(),
            label: "默认系统TTS".to_string(),
        });
        engines.push(TtsEngineInfo {
            package_name: "com.google.android.tts".to_string(),
            label: "Google TTS (Google Text-to-Speech)".to_string(),
        });
    }
    
    engines.sort_by(|a, b| a.label.cmp(&b.label));
    
    Json(ApiResponse::ok(engines))
}

pub async fn speak(Json(req): Json<TtsRequest>) -> Json<ApiResponse<String>> {
    let text = req.text.trim();
    if text.is_empty() {
        return Json(ApiResponse::err("文本内容不能为空"));
    }

    let mut cmd = Command::new("/system/bin/cmd");
    cmd.arg("tts").arg("speak").arg(text);

    if let Some(ref engine) = req.engine {
        cmd.arg("-e").arg(engine);
    }
    if let Some(ref lang) = req.language {
        cmd.arg("-l").arg(lang);
    }
    if let Some(rate) = req.rate {
        cmd.arg("-r").arg(format!("{:.2}", rate));
    }
    if let Some(pitch) = req.pitch {
        cmd.arg("-p").arg(format!("{:.2}", pitch));
    }
    if let Some(volume) = req.volume {
        cmd.arg("-v").arg(format!("{:.2}", volume));
    }

    match cmd.output().await {
        Ok(output) => {
            if output.status.success() {
                return Json(ApiResponse::ok_msg("语音播放成功".to_string(), text));
            }
        }
        Err(e) => {
            return Json(ApiResponse::err(&format!("执行命令失败: {}", e)));
        }
    }

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

pub async fn stop_tts() -> Json<ApiResponse<String>> {
    let result = Command::new("/system/bin/cmd")
        .arg("tts")
        .arg("stop")
        .status()
        .await;

    match result {
        Ok(_) => Json(ApiResponse::ok("语音播放已停止".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("停止失败: {}", e))),
    }
}

#[derive(Debug, serde::Serialize)]
pub struct TtsEngineInfo {
    pub package_name: String,
    pub label: String,
}