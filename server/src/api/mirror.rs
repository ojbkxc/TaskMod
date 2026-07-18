use axum::{extract::ws::WebSocketUpgrade, extract::State, response::IntoResponse, Json};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::Duration;

use crate::data::response::ApiResponse;
use crate::platform::encoder::{
    self, NaluParser, VideoCodec, HEVC_NAL_CRA, HEVC_NAL_IDR_N_LP, HEVC_NAL_IDR_W_RADL,
    HEVC_NAL_PPS, HEVC_NAL_SPS, HEVC_NAL_TRAIL_N, HEVC_NAL_TRAIL_R, HEVC_NAL_VPS,
};
use crate::platform::info;
use crate::platform::input;
use crate::state::SharedMirrorState;

/// 视频广播通道容量（借鉴 RustDesk 多连接缓冲策略，增大以减少 Lagged 断帧）
const VIDEO_CHANNEL_CAP: usize = 128;
/// 音频广播通道容量
const AUDIO_CHANNEL_CAP: usize = 64;
/// H.264/H.265 流读取缓冲区大小（512KB，视频流需要更大缓冲区减少 read 系统调用次数）
#[allow(dead_code)]
const READ_BUF_SIZE: usize = 524288;
/// 默认视频码率 4Mbps（原 10Mbps 在手机上发热严重，4Mbps 在局域网足够清晰）
const DEFAULT_BIT_RATE: usize = 4_000_000;
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
#[allow(dead_code)]
const INPUT_BOOST_FPS: u32 = 60;

// H.265 NALU 类型和编码器类型已移至 platform::encoder 模块

#[derive(Debug, Deserialize)]
pub struct MirrorStartRequest {
    bit_rate: Option<usize>,
    fps: Option<usize>,
    #[allow(dead_code)]
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

    // 跨平台编码器检测（Android: screenrecord, Desktop: ffmpeg 硬件编码器）
    let codec = match req.codec.as_deref() {
        Some("h265") | Some("hevc") => VideoCodec::H265,
        Some("h264") => VideoCodec::H264,
        _ => VideoCodec::detect_best().await,
    };

    let (video_tx, _) = broadcast::channel(VIDEO_CHANNEL_CAP);
    let (audio_tx, _) = broadcast::channel(AUDIO_CHANNEL_CAP);
    let is_running = Arc::new(AtomicBool::new(true));

    // 初始化 ABR 控制器
    state.abr.reset(bit_rate as u32, fps as u32);
    let abr = state.abr.clone();

    let is_running_clone = is_running.clone();
    let request_keyframe_clone = state.request_keyframe.clone();
    let abr_clone = abr.clone();

    // 发送编码器信息给前端（3字节 tag "cdc"，数据为4字节编码器名）
    let codec_tag: &[u8] = match codec {
        VideoCodec::H264 => b"h264",
        VideoCodec::H265 => b"hevc",
    };
    let _ = video_tx.send(encoder::build_nalu_message(b"cdc", codec_tag));

    // 启动跨平台屏幕采集（采集器内部处理编码和 NALU 输出）
    let video_tx_for_capture = video_tx.clone();

    tokio::spawn(async move {
        // 创建采集 channel
        let (capture_tx, mut capture_rx) =
            tokio::sync::mpsc::channel::<crate::platform::capture::CapturedFrame>(64);

        // 启动采集器
        let mut capture = match crate::platform::capture::create_capture(
            codec,
            bit_rate as u32,
            fps as u32,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("[mirror] 采集器创建失败: {}", e);
                return;
            }
        };

        if let Err(e) = capture.start(capture_tx).await {
            tracing::error!("[mirror] 采集器启动失败: {}", e);
            return;
        }

        let mut nalu_parser = NaluParser::new(codec);
        let mut fec_group: Vec<Vec<u8>> = Vec::with_capacity(FEC_GROUP_SIZE);
        let mut fec_group_id: u32 = 0;
        let mut consecutive_static_frames: u32 = 0;
        let mut frame_skip_counter: u32 = 0;

        // 发送 rst 信号
        let _ = video_tx_for_capture.send(encoder::build_nalu_message(b"rst", b""));

        loop {
            if !is_running_clone.load(Ordering::Relaxed) {
                break;
            }

            // ABR 调整
            if abr_clone.take_need_restart() {
                let _ = capture
                    .update_params(
                        abr_clone.get_bitrate(),
                        abr_clone.get_fps(),
                        abr_clone.get_resolution_scale(),
                    )
                    .await;
            }

            // 关键帧请求
            if request_keyframe_clone.swap(false, Ordering::Relaxed) {
                let _ = capture.request_keyframe().await;
            }

            match tokio::time::timeout(Duration::from_millis(100), capture_rx.recv()).await {
                Ok(Some(crate::platform::capture::CapturedFrame::EncodedNalu(data))) => {
                    // 解析 NALU 流
                    let nalus = nalu_parser.feed(&data);
                    for (nalu_type, nalu_data, frame_seq, frame_ts) in nalus {
                        match nalu_type {
                            // SPS
                            7 | HEVC_NAL_SPS => {
                                let _ = video_tx_for_capture
                                    .send(encoder::build_nalu_message(b"sps", &nalu_data));
                            }
                            // PPS
                            8 | HEVC_NAL_PPS => {
                                let _ = video_tx_for_capture
                                    .send(encoder::build_nalu_message(b"pps", &nalu_data));
                            }
                            // VPS (H.265)
                            HEVC_NAL_VPS => {
                                let _ = video_tx_for_capture
                                    .send(encoder::build_nalu_message(b"vps", &nalu_data));
                            }
                            // IDR 关键帧
                            5 | HEVC_NAL_IDR_W_RADL | HEVC_NAL_IDR_N_LP | HEVC_NAL_CRA => {
                                let _ = video_tx_for_capture.send(encoder::build_frame_message(
                                    b"key", &nalu_data, frame_seq, frame_ts,
                                ));
                                if nalu_data.len() <= 65000 {
                                    fec_group.push(nalu_data);
                                    if fec_group.len() >= FEC_GROUP_SIZE {
                                        let fec_data = encoder::generate_fec_xor(&fec_group);
                                        let _ = video_tx_for_capture.send(
                                            encoder::build_fec_message(fec_group_id, &fec_data),
                                        );
                                        fec_group.clear();
                                        fec_group_id = fec_group_id.wrapping_add(1);
                                    }
                                }
                            }
                            // 非 IDR 帧
                            HEVC_NAL_TRAIL_N | HEVC_NAL_TRAIL_R => {
                                // 帧差检测：静止画面跳过
                                if nalu_data.len() < STATIC_FRAME_SIZE_THRESHOLD {
                                    consecutive_static_frames += 1;
                                    if consecutive_static_frames > 3 {
                                        frame_skip_counter += 1;
                                        if !frame_skip_counter.is_multiple_of(STATIC_SKIP_FRAMES) {
                                            continue;
                                        }
                                    }
                                } else {
                                    consecutive_static_frames = 0;
                                    frame_skip_counter = 0;
                                }
                                let _ = video_tx_for_capture.send(encoder::build_frame_message(
                                    b"frm", &nalu_data, frame_seq, frame_ts,
                                ));
                                if nalu_data.len() <= 65000 {
                                    fec_group.push(nalu_data);
                                    if fec_group.len() >= FEC_GROUP_SIZE {
                                        let fec_data = encoder::generate_fec_xor(&fec_group);
                                        let _ = video_tx_for_capture.send(
                                            encoder::build_fec_message(fec_group_id, &fec_data),
                                        );
                                        fec_group.clear();
                                        fec_group_id = fec_group_id.wrapping_add(1);
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Ok(Some(crate::platform::capture::CapturedFrame::RawBgra { .. })) => {
                    // 桌面平台的原始帧（未来可在此处编码）
                    // 当前桌面采集器直接输出 EncodedNalu
                }
                Ok(None) => break,
                Err(_) => continue, // 超时，继续循环
            }
        }
    });

    // 跨平台音频采集任务
    let audio_tx_clone = audio_tx.clone();
    let is_running_clone2 = is_running.clone();
    tokio::spawn(async move {
        use crate::platform::audio::{self, AudioFrame};

        let mut audio_capture = match audio::create_capture(audio::AudioCodec::Pcm).await {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("[mirror] 音频采集器创建失败: {} (不影响视频)", e);
                return;
            }
        };

        let (audio_capture_tx, mut audio_capture_rx) = tokio::sync::mpsc::channel::<AudioFrame>(64);

        if let Err(e) = audio_capture.start(audio_capture_tx).await {
            tracing::warn!("[mirror] 音频采集启动失败: {} (不影响视频)", e);
            return;
        }

        while is_running_clone2.load(Ordering::Relaxed) {
            match audio_capture_rx.recv().await {
                Some(AudioFrame::Pcm(data)) => {
                    let _ = audio_tx_clone.send(data);
                }
                Some(AudioFrame::Opus(data)) => {
                    let _ = audio_tx_clone.send(data);
                }
                None => break,
            }
        }
    });

    state.set_video_tx(video_tx);
    state.set_audio_tx(audio_tx);
    state.set_running(true);

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
        &format!(
            "WebSocket: ws://localhost:{}/ws/mirror | KCP: udp://localhost:{}",
            port,
            port + 1
        ),
    ))
}

/// 获取可用传输协议信息（供前端自动选择最优传输方式）
/// 支持 Cloudflare Tunnels 自动检测
pub async fn transport_info(
    State(_state): State<SharedMirrorState>,
) -> Json<ApiResponse<serde_json::Value>> {
    let port = crate::config::get_listen_port();
    let platform = crate::platform::detect_platform();
    let info = serde_json::json!({
        "websocket": {
            "url": format!("ws://localhost:{}/ws/mirror", port),
            "protocol": "TCP",
            "features": ["tcp_nodelay", "binary_frames", "cloudflare_tunnel_compatible"],
            "latency_class": "medium"
        },
        "kcp": {
            "url": format!("udp://localhost:{}", port + 1),
            "protocol": "UDP",
            "features": ["reliable_udp", "fast_retransmit", "no_head_of_line_blocking"],
            "latency_class": "low",
            "note": "仅支持原生客户端（非浏览器），Cloudflare Tunnel 不支持 UDP"
        },
        "recommended": "kcp",
        "platform": format!("{:?}", platform),
        "tunnel_info": {
            "cloudflare_compatible": true,
            "note": "通过 Cloudflare Tunnel 访问时自动使用 WebSocket（wss://），ABR 自适应降低码率应对高延迟"
        }
    });
    Json(ApiResponse::ok(info))
}

pub async fn stop_mirror(State(state): State<SharedMirrorState>) -> Json<ApiResponse<String>> {
    if !state.is_running() {
        return Json(ApiResponse::err("投屏未运行"));
    }

    state.set_running(false);
    state.clear_video_tx();
    state.clear_audio_tx();

    Json(ApiResponse::ok("投屏已停止".to_string()))
}

pub async fn send_control(
    State(state): State<SharedMirrorState>,
    Json(req): Json<ControlRequest>,
) -> Json<ApiResponse<String>> {
    // 跨平台输入控制器
    let controller = input::create_input_controller();

    // 输入活动加速（借鉴 Sunshine 的 Input Activity Boost）
    if matches!(
        req.action.as_str(),
        "touch" | "touch_down" | "touch_move" | "swipe" | "scroll"
    ) {
        let boost_until = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            + INPUT_BOOST_DURATION_MS;
        state
            .input_boost_until
            .store(boost_until, std::sync::atomic::Ordering::Relaxed);
    }

    let result = match req.action.as_str() {
        "touch" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            controller.tap(x, y).await
        }
        "touch_down" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            state.set_last_touch(Some((x, y)));
            controller.swipe(x, y, x, y, 200).await
        }
        "touch_move" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            if let Some((prev_x, prev_y)) = state.get_last_touch() {
                controller.swipe(prev_x, prev_y, x, y, 50).await
            } else {
                state.set_last_touch(Some((x, y)));
                return Json(ApiResponse::ok("移动完成".to_string()));
            }
        }
        "touch_up" => {
            state.set_last_touch(None);
            return Json(ApiResponse::ok("抬起完成".to_string()));
        }
        "back" => controller.key_event(4).await,
        "home" => controller.key_event(3).await,
        "recents" => controller.key_event(187).await,
        "power" => controller.key_event(26).await,
        "volume_up" => controller.key_event(24).await,
        "volume_down" => controller.key_event(25).await,
        // 媒体控制键
        "media_play" => controller.key_event(85).await,
        "media_pause" => controller.key_event(86).await,
        "media_play_pause" => controller.key_event(85).await, // 85 兼容大部分播放器
        "media_next" => controller.key_event(87).await,
        "media_prev" => controller.key_event(88).await,
        "media_stop" => controller.key_event(86).await, // pause 兼容
        "text" => {
            let text = req.text.unwrap_or_default();
            controller.input_text(&text).await
        }
        "swipe" => {
            let x1 = req.x.unwrap_or(0.0) as i32;
            let y1 = req.y.unwrap_or(0.0) as i32;
            let x2 = req.x2.unwrap_or(0.0) as i32;
            let y2 = req.y2.unwrap_or(0.0) as i32;
            let duration = req.duration.unwrap_or(300);
            controller.swipe(x1, y1, x2, y2, duration).await
        }
        "scroll" => {
            let x = req.x.unwrap_or(0.0) as i32;
            let y = req.y.unwrap_or(0.0) as i32;
            let dy = req.dy.unwrap_or(0.0) as i32;
            controller.swipe(x, y, x, y + dy, 300).await
        }
        "keyevent" => {
            let keycode = req.keycode.or(req.key_code).unwrap_or(0);
            controller.key_event(keycode).await
        }
        "set_clipboard" => {
            let text = req.text.unwrap_or_default();
            controller.set_clipboard(&text).await
        }
        "get_clipboard" => match controller.get_clipboard().await {
            Ok(text) => return Json(ApiResponse::ok(text)),
            Err(e) => return Json(ApiResponse::err(&format!("获取失败: {}", e))),
        },
        "open_notification" | "open_settings" | "collapse_panels" => {
            // 这些是 Android 特有功能，跨平台时静默跳过
            Ok(())
        }
        "screen_on" => controller.key_event(224).await,
        "screen_off" => controller.key_event(223).await,
        _ => Err("未知指令".to_string()),
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
    state
        .abr
        .client_queue_depth
        .store(queue_depth, std::sync::atomic::Ordering::Relaxed);
    state
        .abr
        .client_render_fps
        .store(render_fps, std::sync::atomic::Ordering::Relaxed);
    // 累加丢帧数（ABR 在 adjust 中消费后重置）
    if frames_lost > 0 {
        state
            .abr
            .client_frames_lost
            .fetch_add(frames_lost, std::sync::atomic::Ordering::Relaxed);
    }
    state
        .abr
        .client_pipeline_latency
        .store(pipeline_latency, std::sync::atomic::Ordering::Relaxed);

    // 触发双层 ABR 调整算法
    state.abr.adjust(queue_depth, render_fps);

    Json(ApiResponse::ok("ok".to_string()))
}

/// JPEG 截图 fallback 端点（跨平台：Android screencap / Desktop ffmpeg）
pub async fn screencap_jpeg() -> Json<ApiResponse<String>> {
    use base64::Engine;

    let output = match crate::platform::detect_platform() {
        crate::platform::Platform::Android => {
            tokio::process::Command::new("screencap")
                .arg("-p")
                .output()
                .await
        }
        _ => {
            // 桌面平台：使用 ffmpeg 截取单帧
            tokio::process::Command::new("ffmpeg")
                .args([
                    "-f",
                    "gdigrab",
                    "-i",
                    "desktop",
                    "-frames:v",
                    "1",
                    "-f",
                    "image2",
                    "-c:v",
                    "mjpeg",
                    "pipe:1",
                ])
                .output()
                .await
        }
    };

    match output {
        Ok(o) if o.status.success() && !o.stdout.is_empty() => {
            let b64 = base64::engine::general_purpose::STANDARD.encode(&o.stdout);
            Json(ApiResponse::ok(b64))
        }
        Ok(o) => {
            let err = String::from_utf8_lossy(&o.stderr);
            Json(ApiResponse::err(&format!("截图失败: {}", err)))
        }
        Err(e) => Json(ApiResponse::err(&format!("截图执行失败: {}", e))),
    }
}

// NALU 工具函数已移至 platform::encoder 模块

// ==================== 文件上传 ====================

#[derive(Debug, Deserialize)]
pub struct FileUploadReq {
    pub file_base64: String,
    pub filename: String,
    pub dest_dir: Option<String>,
}

pub async fn upload_file_to_device(Json(req): Json<FileUploadReq>) -> Json<ApiResponse<String>> {
    use base64::Engine;

    // 校验文件名，防止路径穿越和命令注入
    if req.filename.contains("..")
        || req.filename.contains('/')
        || req.filename.contains('\\')
        || req.filename.contains('\'')
        || req.filename.contains('"')
        || req.filename.contains(';')
    {
        return Json(ApiResponse::err("无效的文件名"));
    }

    // 跨平台默认下载目录
    let default_dir = match crate::platform::detect_platform() {
        crate::platform::Platform::Android => "/sdcard/Download".to_string(),
        _ => dirs_next::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| ".".to_string()),
    };
    let dest_dir = req.dest_dir.unwrap_or(default_dir);
    let dest_path = format!("{}/{}", dest_dir, req.filename);

    let bytes = match base64::engine::general_purpose::STANDARD.decode(&req.file_base64) {
        Ok(b) => b,
        Err(e) => return Json(ApiResponse::err(&format!("base64解码失败: {}", e))),
    };

    if let Err(e) = tokio::fs::write(&dest_path, &bytes).await {
        return Json(ApiResponse::err(&format!("写入文件失败: {}", e)));
    }

    Json(ApiResponse::ok(dest_path))
}

// ==================== 剪贴板同步 ====================

#[derive(Debug, Deserialize)]
pub struct ClipboardReq {
    pub text: String,
}

pub async fn get_device_clipboard() -> Json<ApiResponse<String>> {
    let controller = input::create_input_controller();
    match controller.get_clipboard().await {
        Ok(text) => Json(ApiResponse::ok(text)),
        Err(e) => Json(ApiResponse::err(&format!("获取剪贴板失败: {}", e))),
    }
}

pub async fn set_device_clipboard(Json(req): Json<ClipboardReq>) -> Json<ApiResponse<String>> {
    let controller = input::create_input_controller();
    match controller.set_clipboard(&req.text).await {
        Ok(_) => Json(ApiResponse::ok("已设置剪贴板".to_string())),
        Err(e) => Json(ApiResponse::err(&format!("设置剪贴板失败: {}", e))),
    }
}

// ==================== 设备信息获取 ====================

pub async fn get_device_info() -> Json<ApiResponse<serde_json::Value>> {
    let device_info = info::get_device_info().await;
    match serde_json::to_value(&device_info) {
        Ok(v) => Json(ApiResponse::ok(v)),
        Err(e) => Json(ApiResponse::err(&format!("序列化失败: {}", e))),
    }
}
