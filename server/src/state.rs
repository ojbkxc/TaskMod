use std::sync::{Arc, RwLock};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use tokio::sync::broadcast;

/// 自适应码率(ABR)控制器
/// 借鉴 RustDesk 的 VideoQoS 系统，但针对 Android screenrecord 场景优化
/// 通过原子变量实现无锁跨线程参数调整
pub struct AbrController {
    /// 当前目标码率 (bps)
    pub current_bitrate: AtomicU32,
    /// 当前目标帧率
    pub current_fps: AtomicU32,
    /// 需要重启 screenrecord 的标志
    pub need_restart: AtomicBool,
    /// 客户端上报的解码队列深度
    pub client_queue_depth: AtomicU32,
    /// 客户端上报的实际渲染 FPS
    pub client_render_fps: AtomicU32,
    /// 上次 ABR 调整时间戳 (ms since epoch)
    pub last_adjust_time: AtomicU64,
    /// 初始码率（用于恢复）
    pub initial_bitrate: AtomicU32,
    /// 初始帧率
    pub initial_fps: AtomicU32,
    /// 目标分辨率缩放比例 * 100（100=原始，50=半分辨率，75=75%分辨率）
    pub resolution_scale: AtomicU32,
}

impl AbrController {
    pub fn new(bitrate: u32, fps: u32) -> Self {
        Self {
            current_bitrate: AtomicU32::new(bitrate),
            current_fps: AtomicU32::new(fps),
            need_restart: AtomicBool::new(false),
            client_queue_depth: AtomicU32::new(0),
            client_render_fps: AtomicU32::new(0),
            last_adjust_time: AtomicU64::new(0),
            initial_bitrate: AtomicU32::new(bitrate),
            initial_fps: AtomicU32::new(fps),
            resolution_scale: AtomicU32::new(100),
        }
    }

    /// 根据客户端反馈调整码率和帧率（核心 ABR 算法）
    /// 借鉴 RustDesk 的 adjust_ratio() 但更激进，适合局域网场景
    pub fn adjust(&self, queue_depth: u32, render_fps: u32) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = self.last_adjust_time.load(Ordering::Relaxed);
        // 至少间隔 2 秒才调整一次，避免频繁重启 screenrecord
        if now - last < 2000 {
            return;
        }
        self.last_adjust_time.store(now, Ordering::Relaxed);

        let cur_br = self.current_bitrate.load(Ordering::Relaxed);
        let cur_fps = self.current_fps.load(Ordering::Relaxed);
        let mut new_br = cur_br;
        let mut new_fps = cur_fps;
        let mut changed = false;

        // === 码率调整策略 ===
        if queue_depth > 10 {
            // 严重积压：大幅降低码率 30%
            new_br = (cur_br as f64 * 0.7) as u32;
            changed = true;
            tracing::warn!("[ABR] 队列严重积压({})，码率 {} -> {} kbps", queue_depth, cur_br/1000, new_br/1000);
        } else if queue_depth > 6 {
            // 中度积压：降低码率 15%
            new_br = (cur_br as f64 * 0.85) as u32;
            changed = true;
            tracing::info!("[ABR] 队列积压({})，码率 {} -> {} kbps", queue_depth, cur_br/1000, new_br/1000);
        } else if queue_depth > 3 {
            // 轻度积压：降低码率 5%
            new_br = (cur_br as f64 * 0.95) as u32;
            changed = true;
        } else if queue_depth <= 1 && render_fps >= cur_fps.saturating_sub(5) {
            // 队列空闲且客户端能跟上：尝试提升码率 10%（渐进式增长，借鉴 RustDesk）
            let max_br = self.initial_bitrate.load(Ordering::Relaxed) * 2; // 最高不超过初始值2倍
            let candidate = (cur_br as f64 * 1.1) as u32;
            new_br = candidate.min(max_br);
            if new_br != cur_br {
                changed = true;
                tracing::info!("[ABR] 性能充裕，码率 {} -> {} kbps", cur_br/1000, new_br/1000);
            }
        }

        // === 帧率调整策略 ===
        if queue_depth > 8 {
            // 严重积压时降低帧率
            new_fps = (cur_fps.saturating_sub(5)).max(10);
            changed = true;
        } else if queue_depth > 4 {
            new_fps = (cur_fps.saturating_sub(2)).max(15);
            if new_fps != cur_fps { changed = true; }
        } else if queue_depth <= 1 && render_fps >= cur_fps.saturating_sub(3) {
            // 性能充裕时提升帧率
            let max_fps = self.initial_fps.load(Ordering::Relaxed).max(30);
            new_fps = (cur_fps + 2).min(max_fps);
            if new_fps != cur_fps { changed = true; }
        }

        // === 码率范围限制 ===
        new_br = new_br.clamp(1_000_000, 20_000_000); // 1Mbps ~ 20Mbps

        // === 分辨率自适应（超越 RustDesk：根据码率自动缩放分辨率）===
        let cur_scale = self.resolution_scale.load(Ordering::Relaxed);
        let mut new_scale = cur_scale;
        if new_br < 2_000_000 {
            new_scale = 50; // 码率低于2Mbps时降至50%分辨率
        } else if new_br < 4_000_000 {
            new_scale = 75; // 码率低于4Mbps时降至75%分辨率
        } else if new_br >= 6_000_000 && queue_depth <= 2 {
            new_scale = 100; // 码率充裕时恢复原始分辨率
        }
        if new_scale != cur_scale {
            changed = true;
            self.resolution_scale.store(new_scale, Ordering::Relaxed);
            tracing::info!("[ABR] 分辨率缩放 {}% -> {}%", cur_scale, new_scale);
        }

        if changed {
            self.current_bitrate.store(new_br, Ordering::Relaxed);
            self.current_fps.store(new_fps, Ordering::Relaxed);
            self.need_restart.store(true, Ordering::Relaxed);
        }
    }

    /// 检查是否需要重启 screenrecord，如果是则消费该标志
    pub fn take_need_restart(&self) -> bool {
        self.need_restart.swap(false, Ordering::Relaxed)
    }

    pub fn get_bitrate(&self) -> u32 {
        self.current_bitrate.load(Ordering::Relaxed)
    }

    pub fn get_fps(&self) -> u32 {
        self.current_fps.load(Ordering::Relaxed)
    }

    pub fn get_resolution_scale(&self) -> u32 {
        self.resolution_scale.load(Ordering::Relaxed)
    }

    /// 重置 ABR 控制器到新的初始值（投屏启动时调用）
    pub fn reset(&self, bitrate: u32, fps: u32) {
        self.current_bitrate.store(bitrate, Ordering::Relaxed);
        self.current_fps.store(fps, Ordering::Relaxed);
        self.initial_bitrate.store(bitrate, Ordering::Relaxed);
        self.initial_fps.store(fps, Ordering::Relaxed);
        self.need_restart.store(false, Ordering::Relaxed);
        self.client_queue_depth.store(0, Ordering::Relaxed);
        self.client_render_fps.store(0, Ordering::Relaxed);
        self.last_adjust_time.store(0, Ordering::Relaxed);
        self.resolution_scale.store(100, Ordering::Relaxed);
    }
}

pub struct MirrorState {
    pub video_tx: Arc<RwLock<Option<broadcast::Sender<Vec<u8>>>>>,
    pub audio_tx: Arc<RwLock<Option<broadcast::Sender<Vec<u8>>>>>,
    pub is_running: Arc<AtomicBool>,
    pub original_brightness: Arc<RwLock<Option<String>>>,
    pub last_touch: Arc<RwLock<Option<(i32, i32)>>>,
    /// 关键帧请求标志（客户端通过 WebSocket 发送 "keyframe" 消息触发）
    pub request_keyframe: Arc<AtomicBool>,
    /// 自适应码率控制器
    pub abr: Arc<AbrController>,
}

impl MirrorState {
    pub fn new() -> Self {
        Self {
            video_tx: Arc::new(RwLock::new(None)),
            audio_tx: Arc::new(RwLock::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            original_brightness: Arc::new(RwLock::new(None)),
            last_touch: Arc::new(RwLock::new(None)),
            request_keyframe: Arc::new(AtomicBool::new(false)),
            // ABR 默认值，实际在 start_mirror 时会被覆盖
            abr: Arc::new(AbrController::new(8_000_000, 30)),
        }
    }

    pub fn set_video_tx(&self, tx: broadcast::Sender<Vec<u8>>) {
        *self.video_tx.write().unwrap_or_else(|e| {
            tracing::warn!("video_tx 锁中毒，已恢复");
            e.into_inner()
        }) = Some(tx);
    }

    pub fn set_audio_tx(&self, tx: broadcast::Sender<Vec<u8>>) {
        *self.audio_tx.write().unwrap_or_else(|e| {
            tracing::warn!("audio_tx 锁中毒，已恢复");
            e.into_inner()
        }) = Some(tx);
    }

    pub fn set_original_brightness(&self, brightness: String) {
        *self.original_brightness.write().unwrap_or_else(|e| {
            tracing::warn!("original_brightness 锁中毒，已恢复");
            e.into_inner()
        }) = Some(brightness);
    }

    pub fn clear_video_tx(&self) {
        *self.video_tx.write().unwrap_or_else(|e| {
            tracing::warn!("video_tx 锁中毒，已恢复");
            e.into_inner()
        }) = None;
    }

    pub fn clear_audio_tx(&self) {
        *self.audio_tx.write().unwrap_or_else(|e| {
            tracing::warn!("audio_tx 锁中毒，已恢复");
            e.into_inner()
        }) = None;
    }

    pub fn get_video_rx(&self) -> Option<broadcast::Receiver<Vec<u8>>> {
        self.video_tx.read().unwrap_or_else(|e| {
            tracing::warn!("video_tx 锁中毒，已恢复");
            e.into_inner()
        }).as_ref().map(|tx| tx.subscribe())
    }

    pub fn get_audio_rx(&self) -> Option<broadcast::Receiver<Vec<u8>>> {
        self.audio_tx.read().unwrap_or_else(|e| {
            tracing::warn!("audio_tx 锁中毒，已恢复");
            e.into_inner()
        }).as_ref().map(|tx| tx.subscribe())
    }

    pub fn get_original_brightness(&self) -> Option<String> {
        self.original_brightness.read().unwrap_or_else(|e| {
            tracing::warn!("original_brightness 锁中毒，已恢复");
            e.into_inner()
        }).clone()
    }

    pub fn set_running(&self, running: bool) {
        self.is_running.store(running, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_last_touch(&self, pos: Option<(i32, i32)>) {
        *self.last_touch.write().unwrap_or_else(|e| {
            tracing::warn!("last_touch 锁中毒，已恢复");
            e.into_inner()
        }) = pos;
    }

    pub fn get_last_touch(&self) -> Option<(i32, i32)> {
        *self.last_touch.read().unwrap_or_else(|e| {
            tracing::warn!("last_touch 锁中毒，已恢复");
            e.into_inner()
        })
    }

    pub fn request_keyframe(&self) {
        self.request_keyframe.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn take_keyframe_request(&self) -> bool {
        self.request_keyframe.swap(false, std::sync::atomic::Ordering::Relaxed)
    }
}

pub type SharedMirrorState = Arc<MirrorState>;

impl MirrorState {
    pub fn new_shared() -> SharedMirrorState {
        Arc::new(Self::new())
    }
}