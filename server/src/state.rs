use std::sync::{Arc, RwLock};
use std::sync::atomic::AtomicBool;
use tokio::sync::broadcast;

pub struct MirrorState {
    pub video_tx: Arc<RwLock<Option<broadcast::Sender<Vec<u8>>>>>,
    pub audio_tx: Arc<RwLock<Option<broadcast::Sender<Vec<u8>>>>>,
    pub is_running: Arc<AtomicBool>,
    pub original_brightness: Arc<RwLock<Option<String>>>,
}

impl MirrorState {
    pub fn new() -> Self {
        Self {
            video_tx: Arc::new(RwLock::new(None)),
            audio_tx: Arc::new(RwLock::new(None)),
            is_running: Arc::new(AtomicBool::new(false)),
            original_brightness: Arc::new(RwLock::new(None)),
        }
    }

    pub fn set_video_tx(&self, tx: broadcast::Sender<Vec<u8>>) {
        *self.video_tx.write().unwrap() = Some(tx);
    }

    pub fn set_audio_tx(&self, tx: broadcast::Sender<Vec<u8>>) {
        *self.audio_tx.write().unwrap() = Some(tx);
    }

    pub fn set_original_brightness(&self, brightness: String) {
        *self.original_brightness.write().unwrap() = Some(brightness);
    }

    pub fn clear_video_tx(&self) {
        *self.video_tx.write().unwrap() = None;
    }

    pub fn clear_audio_tx(&self) {
        *self.audio_tx.write().unwrap() = None;
    }

    pub fn get_video_rx(&self) -> Option<broadcast::Receiver<Vec<u8>>> {
        self.video_tx.read().unwrap().as_ref().map(|tx| tx.subscribe())
    }

    pub fn get_audio_rx(&self) -> Option<broadcast::Receiver<Vec<u8>>> {
        self.audio_tx.read().unwrap().as_ref().map(|tx| tx.subscribe())
    }

    pub fn get_original_brightness(&self) -> Option<String> {
        self.original_brightness.read().unwrap().clone()
    }

    pub fn set_running(&self, running: bool) {
        self.is_running.store(running, std::sync::Ordering::Relaxed);
    }

    pub fn is_running(&self) -> bool {
        self.is_running.load(std::sync::Ordering::Relaxed)
    }
}

pub type SharedMirrorState = Arc<MirrorState>;

impl MirrorState {
    pub fn new_shared() -> SharedMirrorState {
        Arc::new(Self::new())
    }
}