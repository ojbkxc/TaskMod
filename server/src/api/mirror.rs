use axum::{Json, extract::ws::WebSocketUpgrade, response::IntoResponse};
use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::broadcast;
use tokio::time::Duration;

use crate::data::response::ApiResponse;
use crate::state::SharedMirrorState;

#[derive(Debug, Deserialize)]
pub struct MirrorStartRequest {
    bit_rate: Option<usize>,
    fps: Option<usize>,
    keep_screen: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ControlRequest {
    action: String,
    x: Option<f32>,
    y: Option<f32>,
    x2: Option<f32>,
    y2: Option<f32>,
    key_code: Option<i32>,
    keycode: Option<i32>,
    text: Option<String>,
    dx: Option<f32>,
    dy: Option<f32>,
}

pub async fn start_mirror(
    Json(req): Json<MirrorStartRequest>,
    state: SharedMirrorState,
) -> Json<ApiResponse<String>> {
    let bit_rate = req.bit_rate.unwrap_or(2000000);
    let fps = req.fps.unwrap_or(30);
    let keep_screen = req.keep_screen.unwrap_or(true);

    let original_brightness = if keep_screen {
        let output = Command::new("settings")
            .arg("get")
            .arg("system")
            .arg("screen_brightness")
            .output()
            .await
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let _ = Command::new("settings")
            .arg("put")
            .arg("system")
            .arg("screen_brightness")
            .arg("1")
            .status()
            .await;
        let _ = Command::new("input")
            .arg("keyevent")
            .arg("224")
            .status()
            .await;

        output
    } else {
        None
    };

    let (video_tx, _) = broadcast::channel(32);
    let (audio_tx, _) = broadcast::channel(32);
    let is_running = Arc::new(AtomicBool::new(true));

    let video_tx_clone = video_tx.clone();
    let is_running_clone = is_running.clone();
    tokio::spawn(async move {
        while is_running_clone.load(Ordering::Relaxed) {
            let _ = video_tx_clone.send(build_nalu_message(b"rst", b""));

            let mut cmd = Command::new("screenrecord");
            cmd.arg("--output-format=h264")
                .arg("--bit-rate")
                .arg(bit_rate.to_string())
                .arg("--fps")
                .arg(fps.to_string())
                .arg("-");

            let mut process = match cmd.spawn() {
                Ok(p) => p,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            let stdout = process.stdout.take().unwrap();
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut residual = Vec::new();

            let mut buf = [0u8; 65536];
            loop {
                if !is_running_clone.load(Ordering::Relaxed) {
                    let _ = process.kill().await;
                    break;
                }

                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        residual.extend_from_slice(&buf[..n]);
                        let mut processed = 0;
                        while processed < residual.len() {
                            let start_code = find_nalu_start(&residual[processed..]);
                            if start_code == residual.len() - processed {
                                residual = residual[processed..].to_vec();
                                break;
                            }
                            processed += start_code;

                            let end_pos = find_nalu_end(&residual[processed..]);
                            if end_pos == residual.len() - processed {
                                residual = residual[processed..].to_vec();
                                break;
                            }

                            let nalu_data = &residual[processed..processed + end_pos];
                            processed += end_pos;

                            if nalu_data.is_empty() {
                                continue;
                            }

                            let nalu_type = (nalu_data[0] >> 1) & 0x1F;
                            match nalu_type {
                                7 => {
                                    let _ = video_tx_clone.send(build_nalu_message(b"sps", nalu_data));
                                }
                                8 => {
                                    let _ = video_tx_clone.send(build_nalu_message(b"pps", nalu_data));
                                }
                                1 | 5 => {
                                    let tag = if nalu_type == 5 { b"key" } else { b"frm" };
                                    let _ = video_tx_clone.send(build_nalu_message(tag, nalu_data));
                                }
                                _ => {}
                            }
                        }
                        if processed == residual.len() {
                            residual.clear();
                        } else {
                            residual = residual[processed..].to_vec();
                        }
                    }
                    Err(_) => break,
                }
            }

            let _ = process.wait().await;
            if !is_running_clone.load(Ordering::Relaxed) {
                break;
            }
        }
    });

    let audio_tx_clone = audio_tx.clone();
    let is_running_clone2 = is_running.clone();
    tokio::spawn(async move {
        while is_running_clone2.load(Ordering::Relaxed) {
            let mut cmd = Command::new("tinycap");
            cmd.arg("/dev/stdout")
                .arg("-r")
                .arg("48000")
                .arg("-b")
                .arg("16")
                .arg("-c")
                .arg("1");

            let mut process = match cmd.spawn() {
                Ok(p) => p,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            let stdout = process.stdout.take().unwrap();
            let mut reader = tokio::io::BufReader::new(stdout);
            let mut buf = [0u8; 4096];
            let mut header_buf = Vec::new();
            let mut wav_header_skipped = false;

            loop {
                if !is_running_clone2.load(Ordering::Relaxed) {
                    let _ = process.kill().await;
                    break;
                }

                match reader.read(&mut buf).await {
                    Ok(0) => break,
                    Ok(n) => {
                        if !wav_header_skipped {
                            header_buf.extend_from_slice(&buf[..n]);
                            if header_buf.len() >= 44 {
                                let _ = audio_tx_clone.send(header_buf[44..].to_vec());
                                wav_header_skipped = true;
                            }
                        } else {
                            let _ = audio_tx_clone.send(buf[..n].to_vec());
                        }
                    }
                    Err(_) => break,
                }
            }

            let _ = process.wait().await;
        }
    });

    state.set_video_tx(video_tx);
    state.set_audio_tx(audio_tx);
    state.set_running(true);
    if let Some(b) = original_brightness {
        state.set_original_brightness(b);
    }

    Json(ApiResponse::ok_msg("投屏已启动".to_string(), "WebSocket地址: ws://localhost:9527/ws/mirror"))
}

pub async fn stop_mirror(state: SharedMirrorState) -> Json<ApiResponse<String>> {
    if !state.is_running() {
        return Json(ApiResponse::err("投屏未运行"));
    }

    state.set_running(false);

    if let Some(brightness) = state.get_original_brightness() {
        let _ = Command::new("settings")
            .arg("put")
            .arg("system")
            .arg("screen_brightness")
            .arg(brightness)
            .status()
            .await;
    }

    state.clear_video_tx();
    state.clear_audio_tx();

    Json(ApiResponse::ok("投屏已停止".to_string()))
}

pub async fn send_control(
    Json(req): Json<ControlRequest>,
    _state: SharedMirrorState,
) -> Json<ApiResponse<String>> {
    let result = match req.action.as_str() {
        "touch" | "touch_down" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            Command::new("input")
                .arg("tap")
                .arg(x.to_string())
                .arg(y.to_string())
                .status()
                .await
        }
        "touch_move" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            Command::new("input")
                .arg("swipe")
                .arg(x.to_string())
                .arg(y.to_string())
                .arg(x.to_string())
                .arg(y.to_string())
                .status()
                .await
        }
        "touch_up" => Ok(std::process::ExitStatus::default()),
        "back" => Command::new("input").arg("keyevent").arg("4").status().await,
        "home" => Command::new("input").arg("keyevent").arg("3").status().await,
        "recents" => Command::new("input").arg("keyevent").arg("187").status().await,
        "power" => Command::new("input").arg("keyevent").arg("26").status().await,
        "volume_up" => Command::new("input").arg("keyevent").arg("24").status().await,
        "volume_down" => Command::new("input").arg("keyevent").arg("25").status().await,
        "text" => {
            let text = req.text.unwrap_or_default();
            Command::new("input").arg("text").arg(text).status().await
        }
        "swipe" => {
            let x1 = req.x.unwrap_or(0.0) as i32;
            let y1 = req.y.unwrap_or(0.0) as i32;
            let x2 = req.x2.unwrap_or(0.0) as i32;
            let y2 = req.y2.unwrap_or(0.0) as i32;
            Command::new("input")
                .arg("swipe")
                .arg(x1.to_string())
                .arg(y1.to_string())
                .arg(x2.to_string())
                .arg(y2.to_string())
                .status()
                .await
        }
        "scroll" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            let dx = req.dx.unwrap_or(0.0) as i32;
            let dy = req.dy.unwrap_or(0.0) as i32;
            Command::new("input")
                .arg("scroll")
                .arg(x.to_string())
                .arg(y.to_string())
                .arg(dx.to_string())
                .arg(dy.to_string())
                .status()
                .await
        }
        "keyevent" => {
            let keycode = req.keycode.unwrap_or(0) as i32;
            Command::new("input")
                .arg("keyevent")
                .arg(keycode.to_string())
                .status()
                .await
        }
        "open_notification" => Command::new("cmd").arg("notification").arg("expand").status().await,
        "open_settings" => Command::new("cmd").arg("settings").arg("expand").status().await,
        "collapse_panels" => Command::new("cmd").arg("statusbar").arg("collapse").status().await,
        "set_clipboard" => {
            let text = req.text.unwrap_or_default();
            Command::new("cmd")
                .arg("clipboard")
                .arg("set")
                .arg(text)
                .status()
                .await
        }
        "get_clipboard" => {
            let output = Command::new("cmd")
                .arg("clipboard")
                .arg("get")
                .output()
                .await;
            match output {
                Ok(o) => {
                    let content = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    return Json(ApiResponse::ok(content));
                }
                Err(e) => return Json(ApiResponse::err(&format!("获取失败: {}", e))),
            }
        }
        "screen_on" => Command::new("input").arg("keyevent").arg("26").status().await,
        "screen_off" => Command::new("input").arg("keyevent").arg("26").status().await,
        "start_app" => {
            let pkg = req.text.unwrap_or_default();
            Command::new("monkey")
                .arg("-p")
                .arg(pkg)
                .arg("-c")
                .arg("android.intent.category.LAUNCHER")
                .arg("1")
                .status()
                .await
        }
        "rotate" => Command::new("input").arg("keyevent").arg("186").status().await,
        _ => return Json(ApiResponse::err("未知指令")),
    };

    match result {
        Ok(_) => Json(ApiResponse::ok("指令已发送".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("发送失败: {}", e))),
    }
}

pub async fn mirror_ws(
    ws: WebSocketUpgrade,
    state: SharedMirrorState,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (mut write, _) = socket.split();

        if let Some(mut video_rx) = state.get_video_rx() {
            while let Ok(data) = video_rx.recv().await {
                if let Err(_) = write.send(axum::extract::ws::Message::Binary(data)).await {
                    break;
                }
            }
        }
    })
}

pub async fn audio_ws(
    ws: WebSocketUpgrade,
    state: SharedMirrorState,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {
        let (mut write, _) = socket.split();

        if let Some(mut audio_rx) = state.get_audio_rx() {
            while let Ok(data) = audio_rx.recv().await {
                if let Err(_) = write.send(axum::extract::ws::Message::Binary(data)).await {
                    break;
                }
            }
        }
    })
}

pub async fn mirror_status(state: SharedMirrorState) -> Json<ApiResponse<bool>> {
    Json(ApiResponse::ok(state.is_running()))
}

fn find_nalu_start(data: &[u8]) -> usize {
    for i in 0..data.len().saturating_sub(4) {
        if data[i] == 0 && data[i+1] == 0 && data[i+2] == 0 && data[i+3] == 1 {
            return i + 4;
        }
        if i < data.len().saturating_sub(3) && data[i] == 0 && data[i+1] == 0 && data[i+2] == 1 {
            return i + 3;
        }
    }
    data.len()
}

fn find_nalu_end(data: &[u8]) -> usize {
    for i in 0..data.len().saturating_sub(4) {
        if data[i] == 0 && data[i+1] == 0 && data[i+2] == 0 && data[i+3] == 1 {
            return i;
        }
        if i < data.len().saturating_sub(3) && data[i] == 0 && data[i+1] == 0 && data[i+2] == 1 {
            return i;
        }
    }
    data.len()
}

fn build_nalu_message(tag: &[u8; 3], nalu: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(3 + 4 + nalu.len());
    msg.extend_from_slice(tag);
    msg.extend_from_slice(&(nalu.len() as u32).to_be_bytes());
    msg.extend_from_slice(nalu);
    msg
}