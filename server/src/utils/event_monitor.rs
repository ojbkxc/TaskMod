use std::collections::HashMap;
use std::sync::Mutex;
use tokio::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub struct WifiState {
    pub ssid: String,
    pub connected: bool,
    pub signal_level: i32,
}

#[derive(Debug, Clone)]
pub struct BatteryState {
    pub capacity: i32,
    pub status: String,
    pub temperature: f32,
}

#[derive(Debug, Clone)]
pub struct ScreenState {
    pub on: bool,
    pub brightness: i32,
}

#[derive(Debug, Clone)]
pub enum SystemEvent {
    WifiConnected { ssid: String, signal_level: i32 },
    WifiDisconnected,
    BatteryLow { capacity: i32 },
    BatteryCharging { capacity: i32 },
    BatteryFull,
    ScreenOn,
    ScreenOff,
}

type EventHandler = Box<dyn Fn(SystemEvent) + Send + Sync + 'static>;

static EVENT_HANDLERS: Mutex<Vec<EventHandler>> = Mutex::new(Vec::new());
static LAST_WIFI_STATE: Mutex<Option<WifiState>> = Mutex::new(None);
static LAST_BATTERY_STATE: Mutex<Option<BatteryState>> = Mutex::new(None);
static LAST_SCREEN_STATE: Mutex<Option<ScreenState>> = Mutex::new(None);
static IS_RUNNING: Mutex<bool> = Mutex::new(false);

pub fn register_event_handler<F>(handler: F)
where
    F: Fn(SystemEvent) + Send + Sync + 'static,
{
    EVENT_HANDLERS.lock().unwrap().push(Box::new(handler));
}

fn notify_event(event: SystemEvent) {
    let handlers = EVENT_HANDLERS.lock().unwrap();
    for handler in handlers.iter() {
        handler(event.clone());
    }
}

pub async fn get_wifi_state() -> WifiState {
    let output = match Command::new("dumpsys")
        .arg("wifi")
        .output()
        .await
    {
        Ok(o) => o,
        Err(_) => return WifiState { ssid: "unknown".to_string(), connected: false, signal_level: 0 },
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    
    let mut ssid = "unknown".to_string();
    let mut connected = false;
    let mut signal_level = 0;

    for line in lines {
        if line.contains("mWifiInfo") {
            let parts: Vec<&str> = line.split(',').collect();
            for part in parts {
                if part.contains("SSID:") {
                    let ssid_str = part.split(':').nth(1).unwrap_or("unknown").trim().replace('"', "");
                    if !ssid_str.is_empty() && ssid_str != "<unknown ssid>" {
                        ssid = ssid_str;
                    }
                } else if part.contains("Supplicant state:") {
                    let state = part.split(':').nth(1).unwrap_or("").trim();
                    connected = state == "COMPLETED" || state == "ASSOCIATED";
                } else if part.contains("rssi:") {
                    if let Ok(rssi) = part.split(':').nth(1).unwrap_or("0").trim().parse::<i32>() {
                        signal_level = rssi;
                    }
                }
            }
        }
    }

    if ssid == "unknown" {
        connected = false;
    }

    WifiState { ssid, connected, signal_level }
}

pub async fn get_battery_state() -> BatteryState {
    let output = match Command::new("dumpsys")
        .arg("battery")
        .output()
        .await
    {
        Ok(o) => o,
        Err(_) => return BatteryState { capacity: 0, status: "unknown".to_string(), temperature: 0.0 },
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    
    let mut capacity = 0;
    let mut status = "unknown".to_string();
    let mut temperature = 0.0;

    for line in lines {
        if line.starts_with("level:") {
            if let Ok(level) = line.split(':').nth(1).unwrap_or("0").trim().parse::<i32>() {
                capacity = level;
            }
        } else if line.starts_with("status:") {
            let status_code = line.split(':').nth(1).unwrap_or("0").trim().parse::<i32>().unwrap_or(0);
            status = match status_code {
                1 => "unknown".to_string(),
                2 => "charging".to_string(),
                3 => "discharging".to_string(),
                4 => "not_charging".to_string(),
                5 => "full".to_string(),
                _ => "unknown".to_string(),
            };
        } else if line.starts_with("temperature:") {
            if let Ok(temp) = line.split(':').nth(1).unwrap_or("0").trim().parse::<i32>() {
                temperature = temp as f32 / 10.0;
            }
        }
    }

    BatteryState { capacity, status, temperature }
}

pub async fn get_screen_state() -> ScreenState {
    let output = match Command::new("dumpsys")
        .arg("power")
        .output()
        .await
    {
        Ok(o) => o,
        Err(_) => return ScreenState { on: false, brightness: 0 },
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    
    let mut on = false;
    let mut brightness = 0;

    for line in lines {
        if line.contains("Display Power: state=") {
            let state = line.split("state=").nth(1).unwrap_or("OFF").trim();
            on = state == "ON";
        } else if line.starts_with("mScreenBrightness:") {
            if let Ok(b) = line.split(':').nth(1).unwrap_or("0").trim().parse::<i32>() {
                brightness = b;
            }
        }
    }

    ScreenState { on, brightness }
}

async fn check_wifi_state() {
    let current = get_wifi_state().await;
    let mut last = LAST_WIFI_STATE.lock().unwrap();
    
    match last.as_ref() {
        Some(last_state) => {
            if current.connected && !last_state.connected {
                notify_event(SystemEvent::WifiConnected { ssid: current.ssid.clone(), signal_level: current.signal_level });
            } else if !current.connected && last_state.connected {
                notify_event(SystemEvent::WifiDisconnected);
            } else if current.connected && last_state.connected && current.ssid != last_state.ssid {
                notify_event(SystemEvent::WifiDisconnected);
                notify_event(SystemEvent::WifiConnected { ssid: current.ssid.clone(), signal_level: current.signal_level });
            }
        }
        None => {
            if current.connected {
                notify_event(SystemEvent::WifiConnected { ssid: current.ssid.clone(), signal_level: current.signal_level });
            }
        }
    }
    
    *last = Some(current);
}

async fn check_battery_state() {
    let current = get_battery_state().await;
    let mut last = LAST_BATTERY_STATE.lock().unwrap();
    
    match last.as_ref() {
        Some(last_state) => {
            if current.capacity < 20 && last_state.capacity >= 20 {
                notify_event(SystemEvent::BatteryLow { capacity: current.capacity });
            }
            
            if current.status == "charging" && last_state.status != "charging" {
                notify_event(SystemEvent::BatteryCharging { capacity: current.capacity });
            }
            
            if current.status == "full" && last_state.status != "full" {
                notify_event(SystemEvent::BatteryFull);
            }
        }
        None => {
            if current.capacity < 20 {
                notify_event(SystemEvent::BatteryLow { capacity: current.capacity });
            }
            if current.status == "charging" {
                notify_event(SystemEvent::BatteryCharging { capacity: current.capacity });
            }
            if current.status == "full" {
                notify_event(SystemEvent::BatteryFull);
            }
        }
    }
    
    *last = Some(current);
}

async fn check_screen_state() {
    let current = get_screen_state().await;
    let mut last = LAST_SCREEN_STATE.lock().unwrap();
    
    match last.as_ref() {
        Some(last_state) => {
            if current.on && !last_state.on {
                notify_event(SystemEvent::ScreenOn);
            } else if !current.on && last_state.on {
                notify_event(SystemEvent::ScreenOff);
            }
        }
        None => {
            if current.on {
                notify_event(SystemEvent::ScreenOn);
            }
        }
    }
    
    *last = Some(current);
}

async fn monitor_loop(interval_ms: u64) {
    loop {
        check_wifi_state().await;
        check_battery_state().await;
        check_screen_state().await;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(interval_ms)).await;
        
        let running = IS_RUNNING.lock().unwrap();
        if !*running {
            break;
        }
    }
}

pub fn start_monitor(interval_ms: u64) {
    let mut running = IS_RUNNING.lock().unwrap();
    if *running {
        return;
    }
    *running = true;
    drop(running);
    
    tokio::spawn(async move {
        monitor_loop(interval_ms).await;
    });
}

pub fn stop_monitor() {
    let mut running = IS_RUNNING.lock().unwrap();
    *running = false;
}

pub fn is_monitor_running() -> bool {
    *IS_RUNNING.lock().unwrap()
}

pub fn get_current_states() -> HashMap<String, serde_json::Value> {
    let mut states = HashMap::new();
    
    if let Some(wifi) = LAST_WIFI_STATE.lock().unwrap().as_ref() {
        states.insert("wifi".to_string(), serde_json::json!({
            "ssid": wifi.ssid,
            "connected": wifi.connected,
            "signal_level": wifi.signal_level
        }));
    }
    
    if let Some(battery) = LAST_BATTERY_STATE.lock().unwrap().as_ref() {
        states.insert("battery".to_string(), serde_json::json!({
            "capacity": battery.capacity,
            "status": battery.status,
            "temperature": battery.temperature
        }));
    }
    
    if let Some(screen) = LAST_SCREEN_STATE.lock().unwrap().as_ref() {
        states.insert("screen".to_string(), serde_json::json!({
            "on": screen.on,
            "brightness": screen.brightness
        }));
    }
    
    states
}
