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
        while is_running_clone2.load(Ordering::Relaxed) {
            let mut cmd = Command::new("tinycap");
            cmd.arg("/dev/stdout")
                .arg("-r")
                .arg("48000")
                .arg("-b")
                .arg("16")
                .arg("-c")
                .arg("1")
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
    let result = match req.action.as_str() {
        "touch" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            // 使用tap更可靠，duration设为10ms避免太快被忽略
            Command::new("input")
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
            // 使用短swipe模拟按下，不阻塞input系统
            Command::new("input")
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
                // 从上一个位置滑动到当前位置
                Command::new("input")
                    .arg("swipe")
                    .arg(prev_x.to_string())
                    .arg(prev_y.to_string())
                    .arg(x.to_string())
                    .arg(y.to_string())
                    .arg("50")
                    .status()
                    .await
            } else {
                // 没有之前的位置，直接记录
                state.set_last_touch(Some((x, y)));
                return Json(ApiResponse::ok("移动完成".to_string()));
            }
        }
        "touch_up" => {
            state.set_last_touch(None);
            return Json(ApiResponse::ok("抬起完成".to_string()));
        }
        "back" => Command::new("input").arg("keyevent").arg("4").status().await,
        "home" => Command::new("input").arg("keyevent").arg("3").status().await,
        "recents" => Command::new("input").arg("keyevent").arg("187").status().await,
        "power" => Command::new("input").arg("keyevent").arg("26").status().await,
        "volume_up" => Command::new("input").arg("keyevent").arg("24").status().await,
        "volume_down" => Command::new("input").arg("keyevent").arg("25").status().await,
        "text" => {
            let text = req.text.unwrap_or_default();
            // 对特殊字符进行转义，使用base64方式输入支持中文和空格
            let escaped = text
                .replace("\\", "\\\\")
                .replace(" ", "%s")
                .replace("&", "\\&")
                .replace("<", "\\<")
                .replace(">", "\\>")
                .replace("|", "\\|")
                .replace(";", "\\;")
                .replace("(", "\\(")
                .replace(")", "\\)")
                .replace("'", "\\'")
                .replace("\"", "\\\"");
            Command::new("input").arg("text").arg(escaped).status().await
        }
        "swipe" => {
            let x1 = req.x.unwrap_or(0.0) as i32;
            let y1 = req.y.unwrap_or(0.0) as i32;
            let x2 = req.x2.unwrap_or(0.0) as i32;
            let y2 = req.y2.unwrap_or(0.0) as i32;
            let duration = req.duration.unwrap_or(300);
            Command::new("input")
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
            // scroll is not a valid input command, use swipe instead
            Command::new("input")
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
            // 使用 am broadcast 方式设置剪贴板（兼容性更好）
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
        "screen_on" => Command::new("input").arg("keyevent").arg("224").status().await,
        "screen_off" => Command::new("input").arg("keyevent").arg("223").status().await,
        "start_app" => {
            let pkg = req.text.unwrap_or_default();
            // 先用 cmd package resolve-activity 解析出主Activity名
            let resolve_output = Command::new("sh")
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
            // fallback: 用 monkey 启动
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
            // 切换自动旋转：先关闭自动旋转，再切换方向
            let _ = Command::new("settings")
                .arg("put")
                .arg("system")
                .arg("accelerometer_rotation")
                .arg("0")
                .status()
                .await;
            // 获取当前旋转方向并切换
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

pub async fn mirror_status(State(state): State<SharedMirrorState>) -> Json<ApiResponse<bool>> {
    Json(ApiResponse::ok(state.is_running()))
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