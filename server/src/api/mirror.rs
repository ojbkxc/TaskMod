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

/// 视频广播通道容量（借鉴 RustDesk 多连接缓冲策略，增大以减少 Lagged 断帧）
const VIDEO_CHANNEL_CAP: usize = 128;
/// 音频广播通道容量
const AUDIO_CHANNEL_CAP: usize = 64;
/// H.264/H.265 流读取缓冲区大小（512KB，视频流需要更大缓冲区减少 read 系统调用次数）
const READ_BUF_SIZE: usize = 524288;
/// 默认视频码率 10Mbps（局域网场景，H.265 可以用更高码率获得更好画质）
const DEFAULT_BIT_RATE: usize = 10_000_000;
/// 默认帧率
const DEFAULT_FPS: usize = 60;
/// 静止画面检测阈值：连续 N 帧大小低于此值认为画面静止
const STATIC_FRAME_SIZE_THRESHOLD: usize = 500;
/// 静止画面跳过帧数：检测到静止后每 N 帧只编码一帧
const STATIC_SKIP_FRAMES: u32 = 15;
/// FEC 前向纠错组大小（每N帧生成1个FEC包，20%冗余，借鉴 Sunshine 的 Reed-Solomon 策略）
const FEC_GROUP_SIZE: usize = 5;
/// 输入活动加速持续时间（毫秒）：触摸时临时提升帧率，借鉴 Sunshine 的 Input Activity Boost
const INPUT_BOOST_DURATION_MS: u64 = 300;
/// 输入活动加速时的最低帧率
const INPUT_BOOST_FPS: u32 = 60;

// H.265 NALU 类型常量（与 H.264 的 5-bit 类型不同，H.265 使用 6-bit）
const HEVC_NAL_VPS: u8 = 32;    // Video Parameter Set
const HEVC_NAL_SPS: u8 = 33;    // Sequence Parameter Set
const HEVC_NAL_PPS: u8 = 34;    // Picture Parameter Set
const HEVC_NAL_IDR_W_RADL: u8 = 19; // IDR 帧（带 RADL）
const HEVC_NAL_IDR_N_LP: u8 = 20;   // IDR 帧（无 RADL）
const HEVC_NAL_CRA: u8 = 21;    // Clean Random Access
const HEVC_NAL_TRAIL_R: u8 = 1; // 非IDR参考帧
const HEVC_NAL_TRAIL_N: u8 = 0; // 非IDR非参考帧

/// 视频编码器类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VideoCodec {
    H264,
    H265,
}

impl VideoCodec {
    fn as_str(&self) -> &'static str {
        match self {
            VideoCodec::H264 => "h264",
            VideoCodec::H265 => "hevc",
        }
    }

    /// 检查设备是否支持 H.265 硬件编码
    async fn detect_best_codec() -> Self {
        // 尝试启动 H.265 screenrecord，如果失败则降级到 H.264
        let test = Command::new("screenrecord")
            .args(["--codec=hevc", "--output-format=h264", "--time-limit=1", "--size=1x1", "-"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        match test {
            Ok(mut p) => {
                let _ = p.kill().await;
                let _ = p.wait().await;
                tracing::info!("[mirror] 设备支持 H.265 硬件编码，画质翻倍！");
                VideoCodec::H265
            }
            Err(_) => {
                tracing::info!("[mirror] 设备不支持 H.265，使用 H.264");
                VideoCodec::H264
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MirrorStartRequest {
    bit_rate: Option<usize>,
    fps: Option<usize>,
    keep_screen: Option<bool>,
    /// 编码器偏好: "h264", "h265", "auto"(自动检测)
    codec: Option<String>,
}

/// 客户端上报的网络质量指标（借鉴 RustDesk + Sunshine 的双层 ABR 反馈）
#[derive(Debug, Deserialize)]
pub struct ClientQualityReport {
    /// 前端解码队列积压帧数
    pub queue_depth: Option<u32>,
    /// 客户端实际渲染帧率
    pub render_fps: Option<u32>,
    /// 客户端累计丢帧数（序列号间隙检测）
    pub frames_lost: Option<u32>,
    /// 客户端管线延迟 (ms)
    pub pipeline_latency: Option<u32>,
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
    let bit_rate = req.bit_rate.unwrap_or(DEFAULT_BIT_RATE);
    let fps = req.fps.unwrap_or(DEFAULT_FPS);
    let keep_screen = req.keep_screen.unwrap_or(true);

    // 自动检测最优编码器（H.265 > H.264）
    let codec = match req.codec.as_deref() {
        Some("h265") | Some("hevc") => VideoCodec::H265,
        Some("h264") => VideoCodec::H264,
        _ => VideoCodec::detect_best_codec().await,
    };

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

    let (video_tx, _) = broadcast::channel(VIDEO_CHANNEL_CAP);
    let (audio_tx, _) = broadcast::channel(AUDIO_CHANNEL_CAP);
    let is_running = Arc::new(AtomicBool::new(true));

    // 初始化 ABR 控制器（复用 state 中的实例，通过 reset 更新参数）
    state.abr.reset(bit_rate as u32, fps as u32);
    let abr = state.abr.clone();

    let video_tx_clone = video_tx.clone();
    let is_running_clone = is_running.clone();
    let request_keyframe_clone = state.request_keyframe.clone();
    let abr_clone = abr.clone();
    let input_boost_until = state.input_boost_until.clone();

    // 发送编码器信息给前端（用于解码器配置）
    let codec_tag = match codec {
        VideoCodec::H264 => b"h264",
        VideoCodec::H265 => b"hevc",
    };
    let _ = video_tx.send(build_nalu_message(b"codec", codec_tag));

    tokio::spawn(async move {
        let codec_type = codec;
        while is_running_clone.load(Ordering::Relaxed) {
            let _ = video_tx_clone.send(build_nalu_message(b"rst", b""));

            let cur_bitrate = abr_clone.get_bitrate() as usize;
            // 输入活动加速：触摸时临时提升帧率到60fps
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            let abr_fps = abr_clone.get_fps();
            let cur_fps = if now_ms < input_boost_until.load(Ordering::Relaxed) {
                INPUT_BOOST_FPS.max(abr_fps)
            } else {
                abr_fps
            } as usize;
            let resolution_scale = abr_clone.get_resolution_scale();

            let mut cmd = Command::new("screenrecord");
            // H.265 编码：同等画质码率减半，或同码率画质翻倍
            cmd.arg("--codec").arg(codec_type.as_str())
                .arg("--output-format=h264")  // 输出格式仍为 Annex B
                .arg("--bit-rate")
                .arg(cur_bitrate.to_string())
                .arg("--fps")
                .arg(cur_fps.to_string());

            // 分辨率自适应：当 ABR 系统要求降分辨率时，通过 --size 参数实现
            // 这比单纯降码率更有效，因为编码器处理的像素更少
            if resolution_scale < 100 {
                if let Ok(output) = Command::new("sh").args(["-c", "wm size"]).output().await {
                    let size_str = String::from_utf8_lossy(&output.stdout);
                    if let Some(res) = parse_screen_size(&size_str) {
                        let scaled_w = (res.0 as u32 * resolution_scale / 100) & !1; // 对齐到偶数
                        let scaled_h = (res.1 as u32 * resolution_scale / 100) & !1;
                        cmd.arg("--size").arg(format!("{}x{}", scaled_w, scaled_h));
                    }
                }
            }

            cmd.arg("-").stdout(Stdio::piped());

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
            // 使用更大缓冲区减少 read 系统调用次数
            let mut reader = tokio::io::BufReader::with_capacity(READ_BUF_SIZE, stdout);
            let mut residual = Vec::with_capacity(READ_BUF_SIZE);

            // 帧差检测：跟踪连续小帧数量，静止画面时跳过编码节省带宽
            // 这是 RustDesk 没有的优化——当屏幕内容不变时，P帧极小(<500字节)
            // 检测到静止后每 STATIC_SKIP_FRAMES 帧只发送一帧，节省 90%+ 带宽
            let mut consecutive_static_frames: u32 = 0;
            let mut frame_skip_counter: u32 = 0;

            let mut frame_sequence_counter: u32 = 0;
            let frame_start = std::time::Instant::now();
            // FEC 组缓冲区（XOR 前向纠错，借鉴 Sunshine 的 Reed-Solomon 策略）
            let mut fec_group: Vec<Vec<u8>> = Vec::with_capacity(FEC_GROUP_SIZE);
            let mut fec_group_id: u32 = 0;

            let mut buf = vec![0u8; READ_BUF_SIZE];
            loop {
                if !is_running_clone.load(Ordering::Relaxed) {
                    let _ = process.kill().await;
                    break;
                }

                // 检查关键帧请求：重启 screenrecord 以获取新的 IDR 帧
                if request_keyframe_clone.swap(false, Ordering::Relaxed) {
                    let _ = process.kill().await;
                    break;
                }

                // 检查 ABR 调整：码率/帧率变化时重启 screenrecord
                if abr_clone.take_need_restart() {
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
                                break;
                            }
                            processed += start_code;

                            if processed >= residual.len() {
                                break;
                            }

                            let end_pos = find_nalu_end(&residual[processed..]);
                            if end_pos == residual.len() - processed {
                                break;
                            }

                            if processed + end_pos > residual.len() {
                                break;
                            }

                            let nalu_data = &residual[processed..processed + end_pos];
                            processed += end_pos;

                            if nalu_data.is_empty() {
                                continue;
                            }

                            // 根据编码器类型提取 NALU 类型（H.264: 5-bit, H.265: 6-bit）
                            let nalu_type = if codec_type == VideoCodec::H265 {
                                (nalu_data[0] >> 1) & 0x3F
                            } else {
                                (nalu_data[0] >> 1) & 0x1F
                            };
                            // 帧序列号（单调递增，用于前端丢包检测和 FEC 重组）
                            let frame_seq = frame_sequence_counter;
                            frame_sequence_counter = frame_sequence_counter.wrapping_add(1);
                            let frame_ts = frame_start.elapsed().as_millis() as u32;

                            match nalu_type {
                                // H.264: SPS(7), H.265: SPS(33)
                                7 | HEVC_NAL_SPS => {
                                    let _ = video_tx_clone.send(build_nalu_message(b"sps", nalu_data));
                                }
                                // H.264: PPS(8), H.265: PPS(34)
                                8 | HEVC_NAL_PPS => {
                                    let _ = video_tx_clone.send(build_nalu_message(b"pps", nalu_data));
                                }
                                // H.265 独有：VPS(32)
                                HEVC_NAL_VPS => {
                                    let _ = video_tx_clone.send(build_nalu_message(b"vps", nalu_data));
                                }
                                // H.264: IDR(5), H.265: IDR(19,20) / CRA(21)
                                5 | HEVC_NAL_IDR_W_RADL | HEVC_NAL_IDR_N_LP | HEVC_NAL_CRA => {
                                    let _ = video_tx_clone.send(build_frame_message(b"key", nalu_data, frame_seq, frame_ts));
                                    // FEC 前向纠错：累积帧数据，每 FEC_GROUP_SIZE 帧生成一个 XOR FEC 包
                                    if nalu_data.len() <= 65000 {
                                        fec_group.push(nalu_data.to_vec());
                                        if fec_group.len() >= FEC_GROUP_SIZE {
                                            let fec_data = generate_fec_xor(&fec_group);
                                            let _ = video_tx_clone.send(build_fec_message(fec_group_id, &fec_data));
                                            fec_group.clear();
                                            fec_group_id = fec_group_id.wrapping_add(1);
                                        }
                                    }
                                }
                                // H.264: non-IDR(1), H.265: TRAIL_R(1), TRAIL_N(0)
                                1 | HEVC_NAL_TRAIL_N | HEVC_NAL_TRAIL_R => {
                                    // 帧差检测：P帧且帧极小说明画面静止
                                    if nalu_data.len() < STATIC_FRAME_SIZE_THRESHOLD {
                                        consecutive_static_frames += 1;
                                        // 进入静止模式后，每 STATIC_SKIP_FRAMES 帧只发送一帧
                                        if consecutive_static_frames > 3 {
                                            frame_skip_counter += 1;
                                            if frame_skip_counter % STATIC_SKIP_FRAMES != 0 {
                                                continue; // 跳过此帧
                                            }
                                        }
                                    } else {
                                        consecutive_static_frames = 0;
                                        frame_skip_counter = 0;
                                    }
                                    let _ = video_tx_clone.send(build_frame_message(b"frm", nalu_data, frame_seq, frame_ts));
                                    // FEC 前向纠错：累积帧数据
                                    if nalu_data.len() <= 65000 {
                                        fec_group.push(nalu_data.to_vec());
                                        if fec_group.len() >= FEC_GROUP_SIZE {
                                            let fec_data = generate_fec_xor(&fec_group);
                                            let _ = video_tx_clone.send(build_fec_message(fec_group_id, &fec_data));
                                            fec_group.clear();
                                            fec_group_id = fec_group_id.wrapping_add(1);
                                        }
                                    }
                                }
                                _ => {}
                            }
                        }
                        if processed >= residual.len() {
                            residual.clear();
                        } else if processed > 0 {
                            // 高效移除已处理数据：使用 drain 而非 to_vec 避免额外分配
                            residual.drain(..processed);
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

    // 启动 KCP 服务端（低延迟 UDP 传输）
    let kcp_state = state.clone();
    tokio::spawn(async move {
        let port = crate::config::get_listen_port();
        if let Err(e) = crate::kcp_stream::KcpServer::start(port, kcp_state).await {
            tracing::warn!("[KCP] 服务端启动失败: {} (不影响 WebSocket 传输)", e);
        }
    });

    let port = crate::config::get_listen_port();
    Json(ApiResponse::ok_msg(
        "投屏已启动".to_string(),
        &format!("WebSocket: ws://localhost:{}/ws/mirror | KCP: udp://localhost:{}", port, port + 1),
    ))
}

/// 获取可用传输协议信息（供前端自动选择最优传输方式）
pub async fn transport_info(
    State(_state): State<SharedMirrorState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let port = crate::config::get_listen_port();
    let info = serde_json::json!({
        "websocket": {
            "url": format!("ws://localhost:{}/ws/mirror", port),
            "protocol": "TCP",
            "features": ["tcp_nodelay", "binary_frames"],
            "latency_class": "medium"
        },
        "kcp": {
            "url": format!("udp://localhost:{}", port + 1),
            "protocol": "UDP",
            "features": ["reliable_udp", "fast_retransmit", "no_head_of_line_blocking"],
            "latency_class": "low",
            "note": "仅支持原生客户端（非浏览器）"
        },
        "recommended": "kcp"
    });
    Json(ApiResponse::ok(info))
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

    // 输入活动加速（借鉴 Sunshine 的 Input Activity Boost）：
    // 触摸操作时临时提升帧率到60fps，确保触摸响应丝滑
    if matches!(req.action.as_str(), "touch" | "touch_down" | "touch_move" | "swipe" | "scroll") {
        let boost_until = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64 + INPUT_BOOST_DURATION_MS;
        state.input_boost_until.store(boost_until, std::sync::atomic::Ordering::Relaxed);
    }

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
    let state_clone = state.clone();
    ws.on_upgrade(move |socket| async move {
        let (mut write, mut read) = socket.split();
        let mut ping_interval = tokio::time::interval(Duration::from_secs(30));
        let mut last_pong = tokio::time::Instant::now();
        const PONG_TIMEOUT: Duration = Duration::from_secs(60);

        if let Some(mut video_rx) = state_clone.get_video_rx() {
            loop {
                tokio::select! {
                    biased;
                    msg = read.next() => {
                        match msg {
                            Some(Ok(axum::extract::ws::Message::Pong(_))) => {
                                last_pong = tokio::time::Instant::now();
                            }
                            Some(Ok(axum::extract::ws::Message::Text(text))) => {
                                // 支持客户端发送控制消息
                                if text == "keyframe" {
                                    // 客户端请求关键帧（画面卡住或新连接时）
                                    state_clone.request_keyframe();
                                }
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
                            // 借鉴 RustDesk：Lagged 时跳过积压帧，继续接收最新帧，不断开连接
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                continue;
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
                            Err(broadcast::error::RecvError::Lagged(_)) => {
                                continue;
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

/// 客户端上报网络质量指标（借鉴 RustDesk 的 ABR 反馈机制）
/// 前端定期发送解码队列深度和渲染帧率，后端据此自适应调整码率和帧率
pub async fn report_quality(
    State(state): State<SharedMirrorState>,
    Json(report): Json<ClientQualityReport>,
) -> Json<ApiResponse<String>> {
    let queue_depth = report.queue_depth.unwrap_or(0);
    let render_fps = report.render_fps.unwrap_or(30);
    let frames_lost = report.frames_lost.unwrap_or(0);
    let pipeline_latency = report.pipeline_latency.unwrap_or(0);

    // 更新 ABR 控制器的客户端反馈数据
    state.abr.client_queue_depth.store(queue_depth, std::sync::atomic::Ordering::Relaxed);
    state.abr.client_render_fps.store(render_fps, std::sync::atomic::Ordering::Relaxed);
    // 累加丢帧数（ABR 在 adjust 中消费后重置）
    if frames_lost > 0 {
        state.abr.client_frames_lost.fetch_add(frames_lost, std::sync::atomic::Ordering::Relaxed);
    }
    state.abr.client_pipeline_latency.store(pipeline_latency, std::sync::atomic::Ordering::Relaxed);

    // 触发双层 ABR 调整算法
    state.abr.adjust(queue_depth, render_fps);

    Json(ApiResponse::ok("ok".to_string()))
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

/// 构建带序列号和时间戳的帧消息（借鉴 Sunshine 的短帧头设计）
/// 格式: [tag(3B)][len(4B)][seq(4B)][timestamp_ms(4B)][data]
fn build_frame_message(tag: &[u8; 3], nalu: &[u8], seq: u32, ts: u32) -> Vec<u8> {
    let data_len = 8 + nalu.len(); // seq(4) + ts(4) + nalu
    let mut msg = Vec::with_capacity(3 + 4 + data_len);
    msg.extend_from_slice(tag);
    msg.extend_from_slice(&(data_len as u32).to_be_bytes());
    msg.extend_from_slice(&seq.to_be_bytes());
    msg.extend_from_slice(&ts.to_be_bytes());
    msg.extend_from_slice(nalu);
    msg
}

/// 构建 FEC 前向纠错消息
/// 格式: [tag="fec"(3B)][len(4B)][group_id(4B)][fec_data]
fn build_fec_message(group_id: u32, fec_data: &[u8]) -> Vec<u8> {
    let data_len = 4 + fec_data.len();
    let mut msg = Vec::with_capacity(3 + 4 + data_len);
    msg.extend_from_slice(b"fec");
    msg.extend_from_slice(&(data_len as u32).to_be_bytes());
    msg.extend_from_slice(&group_id.to_be_bytes());
    msg.extend_from_slice(fec_data);
    msg
}

/// 生成 XOR 前向纠错数据（所有帧异或，可恢复组内任意单帧丢失）
fn generate_fec_xor(frames: &[Vec<u8>]) -> Vec<u8> {
    if frames.is_empty() {
        return Vec::new();
    }
    let max_len = frames.iter().map(|f| f.len()).max().unwrap_or(0);
    let mut fec = vec![0u8; max_len];
    for frame in frames {
        for (i, &byte) in frame.iter().enumerate() {
            fec[i] ^= byte;
        }
    }
    fec
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
    // 获取CPU使用率
    if let Ok(output) = Command::new("sh").args(["-c", "top -bn1 | head -3 | grep 'CPU' | awk '{print $2}'"]).output().await {
        let cpu = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !cpu.is_empty() {
            info.insert("cpu".into(), serde_json::Value::String(cpu));
        }
    }
    // 获取内存信息
    if let Ok(output) = Command::new("sh").args(["-c", "cat /proc/meminfo | grep -E 'MemTotal|MemAvailable' | awk '{printf \"%s \", $2}'"]).output().await {
        let mem_raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = mem_raw.split_whitespace().collect();
        if parts.len() >= 2 {
            if let (Ok(total), Ok(avail)) = (parts[0].parse::<u64>(), parts[1].parse::<u64>()) {
                let used_mb = (total - avail) / 1024;
                let total_mb = total / 1024;
                info.insert("memory".into(), serde_json::Value::String(format!("{}MB/{}MB", used_mb, total_mb)));
            }
        }
    }
    // 获取WiFi SSID
    if let Ok(output) = Command::new("sh").args(["-c", "dumpsys wifi | grep 'mWifiInfo' | grep -o 'SSID: [^,]*' | head -1"]).output().await {
        let wifi = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !wifi.is_empty() {
            info.insert("wifi".into(), serde_json::Value::String(wifi));
        }
    }

    Json(ApiResponse::ok(serde_json::Value::Object(info)))
}

/// 解析 `wm size` 输出，如 "Physical size: 1080x1920" 或 "Override size: 1080x2400"
fn parse_screen_size(output: &str) -> Option<(u32, u32)> {
    for line in output.lines() {
        if let Some(pos) = line.rfind(|c: char| c.is_ascii_digit()) {
            // 找到类似 "1080x1920" 的部分
            let segment = &line[..=pos];
            if let Some(x_pos) = segment.rfind('x') {
                let w_str = &segment[..x_pos];
                let h_str = &segment[x_pos + 1..];
                if let (Some(w_start), _) = (w_str.rfind(|c: char| !c.is_ascii_digit()), ()) {
                    if let (Ok(w), Ok(h)) = (w_str[w_start + 1..].parse::<u32>(), h_str.parse::<u32>()) {
                        if w > 0 && h > 0 {
                            return Some((w, h));
                        }
                    }
                } else if let (Ok(w), Ok(h)) = (w_str.parse::<u32>(), h_str.parse::<u32>()) {
                    if w > 0 && h > 0 {
                        return Some((w, h));
                    }
                }
            }
        }
    }
    None
}