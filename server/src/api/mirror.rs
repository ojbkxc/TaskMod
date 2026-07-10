use axum::{Json, extract::State, extract::ws::WebSocketUpgrade, response::IntoResponse};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use std::process::Stdio;
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
#[allow(dead_code)]
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
    duration: Option<u64>,
}

pub async fn start_mirror(
    State(state): State<SharedMirrorState>,
    Json(req): Json<MirrorStartRequest>,
) -> Json<ApiResponse<String>> {
    let bit_rate = req.bit_rate.unwrap_or(2000000);
    let _fps = req.fps.unwrap_or(30);
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
                .arg("-")
                .stdout(Stdio::piped());

            let mut process = match cmd.spawn() {
                Ok(p) => p,
                Err(_) => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };

            let stdout = match process.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = process.kill().await;
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    continue;
                }
            };
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
                            // 查找当前 NALU 的起始码
                            let start_code = find_nalu_start(&residual[processed..]);
                            if start_code == residual.len() - processed {
                                // 没有找到完整的起始码，保留残余数据等待下次读取
                                break;
                            }
                            processed += start_code;

                            // 边界保护
                            if processed >= residual.len() {
                                break;
                            }

                            // 查找下一个 NALU 的起始码（即当前 NALU 的结束位置）
                            let end_pos = find_nalu_end(&residual[processed..]);
                            if end_pos == residual.len() - processed {
                                // 没有找到结束位置，保留残余数据等待下次读取
                                break;
                            }

                            // 边界保护
                            if processed + end_pos > residual.len() {
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
                        if processed >= residual.len() {
                            residual.clear();
                        } else if processed > 0 {
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
        let mut consecutive_failures = 0u32;
        const MAX_FAILURES: u32 = 3;

        while is_running_clone2.load(Ordering::Relaxed) {
            // 连续失败超过阈值后停止重试，等待下次手动触发
            if consecutive_failures >= MAX_FAILURES {
                eprintln!("[mirror] tinycap 连续失败{}次，音频录制已停止（设备可能无麦克风或权限不足）", MAX_FAILURES);
                break;
            }

            let mut cmd = Command::new("tinycap");
            cmd.arg("/dev/stdout")
                .arg("-r")
                .arg("48000")
                .arg("-b")
                .arg("16")
                .arg("-c")
                .arg("1")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut process = match cmd.spawn() {
                Ok(p) => p,
                Err(e) => {
                    consecutive_failures += 1;
                    eprintln!("[mirror] tinycap 启动失败 ({}): {}", consecutive_failures, e);
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            // 检查 tinycap 是否立即退出（PCM设备不存在）
            tokio::time::sleep(Duration::from_millis(100)).await;
            if let Ok(Some(status)) = process.try_wait() {
                consecutive_failures += 1;
                let stderr = process.stderr.take();
                if let Some(mut stderr) = stderr {
                    let mut err_msg = String::new();
                    use tokio::io::AsyncReadExt;
                    let _ = stderr.read_to_string(&mut err_msg).await;
                    eprintln!("[mirror] tinycap 退出 ({}): {} - {}", consecutive_failures, status, err_msg.trim());
                } else {
                    eprintln!("[mirror] tinycap 退出 ({}): {}", consecutive_failures, status);
                }
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }

            let stdout = match process.stdout.take() {
                Some(s) => s,
                None => {
                    let _ = process.kill().await;
                    consecutive_failures += 1;
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    continue;
                }
            };

            // 成功启动，重置失败计数
            consecutive_failures = 0;
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
                            // WAV头长度不固定，查找"data"标记来确定头部结束位置
                            if let Some(pos) = find_wav_data_offset(&header_buf) {
                                let _ = audio_tx_clone.send(header_buf[pos..].to_vec());
                                wav_header_skipped = true;
                            } else if header_buf.len() > 4096 {
                                // 头部过大，可能不是WAV格式，直接发送原始数据
                                let _ = audio_tx_clone.send(header_buf.clone());
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

pub async fn stop_mirror(State(state): State<SharedMirrorState>) -> Json<ApiResponse<String>> {
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
    State(state): State<SharedMirrorState>,
    Json(req): Json<ControlRequest>,
) -> Json<ApiResponse<String>> {
    // Android 上 input 等命令可能不在 PATH 中，尝试常见路径
    let input_cmd = if std::path::Path::new("/system/bin/input").exists() {
        "/system/bin/input"
    } else {
        "input"
    };
    let sh_cmd = if std::path::Path::new("/system/bin/sh").exists() {
        "/system/bin/sh"
    } else {
        "sh"
    };

    let result = match req.action.as_str() {
        "touch" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            Command::new(input_cmd)
                .arg("tap")
                .arg(x.to_string())
                .arg(y.to_string())
                .status()
                .await
        }
        "touch_down" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            state.set_last_touch(Some((x, y)));
            Command::new(input_cmd)
                .arg("swipe")
                .arg(x.to_string())
                .arg(y.to_string())
                .arg(x.to_string())
                .arg(y.to_string())
                .arg("200")
                .status()
                .await
        }
        "touch_move" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            if let Some((prev_x, prev_y)) = state.get_last_touch() {
                Command::new(input_cmd)
                    .arg("swipe")
                    .arg(prev_x.to_string())
                    .arg(prev_y.to_string())
                    .arg(x.to_string())
                    .arg(y.to_string())
                    .arg("50")
                    .status()
                    .await
            } else {
                state.set_last_touch(Some((x, y)));
                return Json(ApiResponse::ok("移动完成".to_string()));
            }
        }
        "touch_up" => {
            state.set_last_touch(None);
            return Json(ApiResponse::ok("抬起完成".to_string()));
        }
        "back" => Command::new(input_cmd).arg("keyevent").arg("4").status().await,
        "home" => Command::new(input_cmd).arg("keyevent").arg("3").status().await,
        "recents" => Command::new(input_cmd).arg("keyevent").arg("187").status().await,
        "power" => Command::new(input_cmd).arg("keyevent").arg("26").status().await,
        "volume_up" => Command::new(input_cmd).arg("keyevent").arg("24").status().await,
        "volume_down" => Command::new(input_cmd).arg("keyevent").arg("25").status().await,
        "text" => {
            let text = req.text.unwrap_or_default();
            let escaped = text
                .replace("\\", "\\\\")
                .replace("\"", "\\\"")
                .replace("'", "\\'")
                .replace(" ", "%s")
                .replace("&", "\\&")
                .replace("<", "\\<")
                .replace(">", "\\>")
                .replace("|", "\\|")
                .replace(";", "\\;")
                .replace("(", "\\(")
                .replace(")", "\\)");
            Command::new(input_cmd).arg("text").arg(escaped).status().await
        }
        "swipe" => {
            let x1 = req.x.unwrap_or(0.0) as i32;
            let y1 = req.y.unwrap_or(0.0) as i32;
            let x2 = req.x2.unwrap_or(0.0) as i32;
            let y2 = req.y2.unwrap_or(0.0) as i32;
            let duration = req.duration.unwrap_or(300);
            Command::new(input_cmd)
                .arg("swipe")
                .arg(x1.to_string())
                .arg(y1.to_string())
                .arg(x2.to_string())
                .arg(y2.to_string())
                .arg(duration.to_string())
                .status()
                .await
        }
        "scroll" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            let dy = req.dy.unwrap_or(0.0) as i32;
            Command::new(input_cmd)
                .arg("swipe")
                .arg(x.to_string())
                .arg(y.to_string())
                .arg(x.to_string())
                .arg((y + dy).to_string())
                .arg("300")
                .status()
                .await
        }
        "keyevent" => {
            let keycode = req.keycode.or(req.key_code).unwrap_or(0) as i32;
            Command::new(input_cmd)
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
            Command::new("am")
                .arg("broadcast")
                .arg("-a")
                .arg("clipper.set")
                .arg("--es")
                .arg("text")
                .arg(&text)
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
        "screen_on" => Command::new(input_cmd).arg("keyevent").arg("224").status().await,
        "screen_off" => Command::new(input_cmd).arg("keyevent").arg("223").status().await,
        "start_app" => {
            let pkg = req.text.unwrap_or_default();
            let resolve_output = Command::new(sh_cmd)
                .arg("-c")
                .arg(format!("cmd package resolve-activity --brief {} | tail -1", &pkg))
                .output()
                .await;
            if let Ok(o) = resolve_output {
                let activity = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !activity.is_empty() && !activity.contains("Error") && activity.contains('/') {
                    let am_result = Command::new("am")
                        .arg("start")
                        .arg("-n")
                        .arg(&activity)
                        .status()
                        .await;
                    if let Ok(s) = am_result {
                        if s.success() {
                            return Json(ApiResponse::ok(format!("应用已启动: {}", activity)));
                        }
                    }
                }
            }
            Command::new("monkey")
                .arg("-p")
                .arg(pkg)
                .arg("-c")
                .arg("android.intent.category.LAUNCHER")
                .arg("1")
                .status()
                .await
        }
        "rotate" => {
            let _ = Command::new("settings")
                .arg("put")
                .arg("system")
                .arg("accelerometer_rotation")
                .arg("0")
                .status()
                .await;
            let current = Command::new("settings")
                .arg("get")
                .arg("system")
                .arg("user_rotation")
                .output()
                .await
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .and_then(|s| s.trim().parse::<i32>().ok())
                .unwrap_or(0);
            let next = (current + 1) % 4;
            Command::new("settings")
                .arg("put")
                .arg("system")
                .arg("user_rotation")
                .arg(next.to_string())
                .status()
                .await
        }
        "stop_app" => {
            let pkg = req.text.unwrap_or_default();
            Command::new("am")
                .arg("force-stop")
                .arg(pkg)
                .status()
                .await
        }
        _ => return Json(ApiResponse::err("未知指令")),
    };

    match result {
        Ok(_) => Json(ApiResponse::ok("指令已发送".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("发送失败: {}", e))),
    }
}

pub async fn mirror_ws(
    State(state): State<SharedMirrorState>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    if crate::WS_CONNECTION_COUNT.load(Ordering::Relaxed) >= crate::MAX_WS_CONNECTIONS {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    crate::WS_CONNECTION_COUNT.fetch_add(1, Ordering::Relaxed);
    ws.on_upgrade(move |socket| async move {
        let (mut write, mut read) = socket.split();
        let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
        let mut last_pong = tokio::time::Instant::now();
        const PONG_TIMEOUT: Duration = Duration::from_secs(60);

        if let Some(mut video_rx) = state.get_video_rx() {
            loop {
                tokio::select! {
                    biased;
                    msg = read.next() => {
                        match msg {
                            Some(Ok(axum::extract::ws::Message::Pong(_))) => {
                                last_pong = tokio::time::Instant::now();
                            }
                            Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                    _ = ping_interval.tick() => {
                        if last_pong.elapsed() > PONG_TIMEOUT {
                            tracing::warn!("[mirror_ws] 客户端心跳超时，关闭连接");
                            break;
                        }
                        if write.send(axum::extract::ws::Message::Ping(vec![])).await.is_err() {
                            break;
                        }
                    }
                    data = video_rx.recv() => {
                        match data {
                            Ok(data) => {
                                if write.send(axum::extract::ws::Message::Binary(data)).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        }
        crate::WS_CONNECTION_COUNT.fetch_sub(1, Ordering::Relaxed);
    })
}

/// 查找WAV文件中"data"标记的位置，返回音频数据的起始偏移量
fn find_wav_data_offset(data: &[u8]) -> Option<usize> {
    // WAV格式: RIFF header + fmt chunk + data chunk
    // 查找 "data" 标记 (0x64617461)
    for i in 0..data.len().saturating_sub(4) {
        if data[i] == b'd' && data[i+1] == b'a' && data[i+2] == b't' && data[i+3] == b'a' {
            // "data" 后面跟着4字节的数据大小，然后是实际音频数据
            if i + 8 <= data.len() {
                return Some(i + 8);
            }
        }
    }
    None
}

pub async fn audio_ws(
    State(state): State<SharedMirrorState>,
    ws: WebSocketUpgrade,
) -> axum::response::Response {
    if crate::WS_CONNECTION_COUNT.load(Ordering::Relaxed) >= crate::MAX_WS_CONNECTIONS {
        return axum::http::StatusCode::SERVICE_UNAVAILABLE.into_response();
    }
    crate::WS_CONNECTION_COUNT.fetch_add(1, Ordering::Relaxed);
    ws.on_upgrade(move |socket| async move {
        let (mut write, mut read) = socket.split();
        let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
        let mut last_pong = tokio::time::Instant::now();
        const PONG_TIMEOUT: Duration = Duration::from_secs(60);

        if let Some(mut audio_rx) = state.get_audio_rx() {
            loop {
                tokio::select! {
                    biased;
                    msg = read.next() => {
                        match msg {
                            Some(Ok(axum::extract::ws::Message::Pong(_))) => {
                                last_pong = tokio::time::Instant::now();
                            }
                            Some(Ok(axum::extract::ws::Message::Close(_))) | None => break,
                            _ => {}
                        }
                    }
                    _ = ping_interval.tick() => {
                        if last_pong.elapsed() > PONG_TIMEOUT {
                            tracing::warn!("[audio_ws] 客户端心跳超时，关闭连接");
                            break;
                        }
                        if write.send(axum::extract::ws::Message::Ping(vec![])).await.is_err() {
                            break;
                        }
                    }
                    data = audio_rx.recv() => {
                        match data {
                            Ok(data) => {
                                if write.send(axum::extract::ws::Message::Binary(data)).await.is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                }
            }
        }
        crate::WS_CONNECTION_COUNT.fetch_sub(1, Ordering::Relaxed);
    })
}

pub async fn mirror_status(State(state): State<SharedMirrorState>) -> Json<ApiResponse<bool>> {
    Json(ApiResponse::ok(state.is_running()))
}

/// JPEG 截图 fallback 端点
pub async fn screencap_jpeg() -> Json<ApiResponse<String>> {
    use base64::Engine;
    let output = Command::new("screencap")
        .arg("-p")
        .output()
        .await;

    match output {
        Ok(o) if o.status.success() && !o.stdout.is_empty() => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&o.stdout);
            Json(ApiResponse::ok(b64))
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            Json(ApiResponse::err(&format!("screencap 失败: {}", err)))
        }
        Err(e) => Json(ApiResponse::err(&format!("screencap 执行失败: {}", e))),
    }
}

fn find_nalu_start(data: &[u8]) -> usize {
    if data.len() < 3 {
        return data.len();
    }
    for i in 0..=data.len() - 3 {
        if data[i] == 0 && data[i + 1] == 0 {
            if data[i + 2] == 1 {
                return i + 3;
            }
            if i + 3 < data.len() && data[i + 2] == 0 && data[i + 3] == 1 {
                return i + 4;
            }
        }
    }
    data.len()
}

fn find_nalu_end(data: &[u8]) -> usize {
    if data.len() < 3 {
        return data.len();
    }
    for i in 1..=data.len() - 3 {
        if data[i] == 0 && data[i + 1] == 0 {
            if data[i + 2] == 1 {
                return i;
            }
            if i + 3 < data.len() && data[i + 2] == 0 && data[i + 3] == 1 {
                return i;
            }
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

// ==================== 文件上传到设备 (借鉴QtScrcpy) ====================

#[derive(Debug, Deserialize)]
pub struct FileUploadReq {
    pub file_base64: String,
    pub filename: String,
    pub dest_dir: Option<String>,
}

pub async fn upload_file_to_device(Json(req): Json<FileUploadReq>) -> Json<ApiResponse<String>> {
    use base64::Engine;

    // 校验文件名，防止路径穿越和命令注入
    if req.filename.contains("..") || req.filename.contains('/') || req.filename.contains('\\')
        || req.filename.contains('\'') || req.filename.contains('"') || req.filename.contains(';')
    {
        return Json(ApiResponse::err("无效的文件名"));
    }

    let dest_dir = req.dest_dir.unwrap_or_else(|| "/sdcard/Download".to_string());
    let dest_path = format!("{}/{}", dest_dir, req.filename);

    // 解码base64
    let bytes = match base64::engine::general_purpose::STANDARD.decode(&req.file_base64) {
        Ok(b) => b,
        Err(e) => return Json(ApiResponse::err(&format!("base64解码失败: {}", e))),
    };

    // 写入临时文件
    let tmp_path = format!("/data/local/tmp/{}", req.filename);
    if let Err(e) = tokio::fs::write(&tmp_path, &bytes).await {
        return Json(ApiResponse::err(&format!("写入临时文件失败: {}", e)));
    }

    // adb push 到设备
    match Command::new("sh")
        .args(["-c", &format!("cp '{}' '{}'", tmp_path, dest_path)])
        .output()
        .await
    {
        Ok(output) => {
            let _ = tokio::fs::remove_file(&tmp_path).await;
            if output.status.success() {
                Json(ApiResponse::ok(dest_path))
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                Json(ApiResponse::err(&format!("推送失败: {}", stderr)))
            }
        }
        Err(e) => Json(ApiResponse::err(&format!("执行失败: {}", e))),
    }
}

// ==================== 剪贴板同步 (借鉴QtScrcpy) ====================

#[derive(Debug, Deserialize)]
pub struct ClipboardReq {
    pub text: String,
}

pub async fn get_device_clipboard() -> Json<ApiResponse<String>> {
    let cmd_output = Command::new("cmd")
        .arg("clipboard")
        .arg("get")
        .output()
        .await;
    
    match cmd_output {
        Ok(output) => {
            let mut text = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if text.is_empty() || text.contains("Error") {
                let fallback = Command::new("sh")
                    .args(["-c", "content query --uri content://clipboard --projection text"])
                    .output()
                    .await;
                if let Ok(fb_output) = fallback {
                    let fb_text = String::from_utf8_lossy(&fb_output.stdout);
                    let parts: Vec<&str> = fb_text.split('=').collect();
                    if parts.len() > 1 {
                        text = parts[1].trim().to_string();
                    }
                }
            }
            if text.starts_with("0x") {
                text = hex_to_string(&text);
            }
            Json(ApiResponse::ok(text))
        }
        Err(e) => {
            let fallback = Command::new("sh")
                .args(["-c", "content query --uri content://clipboard --projection text | cut -d'=' -f2"])
                .output()
                .await;
            match fallback {
                Ok(output) => {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    Json(ApiResponse::ok(text))
                }
                Err(e2) => Json(ApiResponse::err(&format!("获取剪贴板失败: {}, 备用方案也失败: {}", e, e2))),
            }
        }
    }
}

pub async fn set_device_clipboard(Json(req): Json<ClipboardReq>) -> Json<ApiResponse<String>> {
    let text = req.text.clone();
    
    let cmd_output = Command::new("cmd")
        .arg("clipboard")
        .arg("set")
        .arg(&text)
        .output()
        .await;
    
    match cmd_output {
        Ok(output) => {
            if output.status.success() {
                Json(ApiResponse::ok("已设置设备剪贴板".to_string()))
            } else {
                let fallback = Command::new("sh")
                    .args(["-c", &format!("am broadcast -a clipper.set -e text '{}'", escape_for_shell(&text))])
                    .output()
                    .await;
                match fallback {
                    Ok(fb_output) => {
                        if fb_output.status.success() {
                            Json(ApiResponse::ok("已设置设备剪贴板".to_string()))
                        } else {
                            let content_output = Command::new("sh")
                                .args(["-c", &format!("content insert --uri content://clipboard --bind text:s:'{}'", escape_for_shell(&text))])
                                .output()
                                .await;
                            match content_output {
                                Ok(co) => {
                                    if co.status.success() {
                                        Json(ApiResponse::ok("已设置设备剪贴板".to_string()))
                                    } else {
                                        Json(ApiResponse::err(&format!("设置剪贴板失败: {}", String::from_utf8_lossy(&co.stderr))))
                                    }
                                }
                                Err(e) => Json(ApiResponse::err(&format!("设置剪贴板失败: {}", e))),
                            }
                        }
                    }
                    Err(e) => Json(ApiResponse::err(&format!("设置剪贴板失败: {}", e))),
                }
            }
        }
        Err(_e) => {
            let fallback = Command::new("sh")
                .args(["-c", &format!("am broadcast -a clipper.set -e text '{}'", escape_for_shell(&text))])
                .output()
                .await;
            match fallback {
                Ok(fb_output) => {
                    if fb_output.status.success() {
                        Json(ApiResponse::ok("已设置设备剪贴板".to_string()))
                    } else {
                        Json(ApiResponse::err(&format!("设置剪贴板失败: {}", String::from_utf8_lossy(&fb_output.stderr))))
                    }
                }
                Err(e2) => Json(ApiResponse::err(&format!("设置剪贴板失败: {}", e2))),
            }
        }
    }
}

fn hex_to_string(hex: &str) -> String {
    let hex = hex.trim_start_matches("0x");
    let mut result = String::new();
    let bytes = hex.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        if let (Some(&a), Some(&b)) = (bytes.get(i), bytes.get(i + 1)) {
            let byte = match (hex_char_to_u8(a), hex_char_to_u8(b)) {
                (Some(h), Some(l)) => (h << 4) | l,
                _ => continue,
            };
            result.push(byte as char);
        }
    }
    result
}

fn hex_char_to_u8(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

fn escape_for_shell(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('\'', "'\\''")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

// ==================== 设备信息获取 ====================

pub async fn get_device_info() -> Json<ApiResponse<serde_json::Value>> {
    let mut info = serde_json::Map::new();

    // 获取设备型号
    if let Ok(output) = Command::new("sh").args(["-c", "getprop ro.product.model"]).output().await {
        info.insert("model".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stdout).trim().to_string()));
    }
    // 获取Android版本
    if let Ok(output) = Command::new("sh").args(["-c", "getprop ro.build.version.release"]).output().await {
        info.insert("android_version".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stdout).trim().to_string()));
    }
    // 获取屏幕分辨率
    if let Ok(output) = Command::new("sh").args(["-c", "wm size"]).output().await {
        info.insert("screen_size".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stdout).trim().to_string()));
    }
    // 获取电池信息
    if let Ok(output) = Command::new("sh").args(["-c", "dumpsys battery | grep level"]).output().await {
        info.insert("battery".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stdout).trim().to_string()));
    }
    // 获取IP地址
    if let Ok(output) = Command::new("sh").args(["-c", "ip addr show wlan0 | grep 'inet ' | awk '{print $2}'"]).output().await {
        info.insert("ip".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stdout).trim().to_string()));
    }
    // 获取存储空间
    if let Ok(output) = Command::new("sh").args(["-c", "df -h /sdcard | tail -1 | awk '{print \"Used:\"$3\"/Total:\"$2}'"]).output().await {
        info.insert("storage".into(), serde_json::Value::String(String::from_utf8_lossy(&output.stdout).trim().to_string()));
    }

    Json(ApiResponse::ok(serde_json::Value::Object(info)))
}