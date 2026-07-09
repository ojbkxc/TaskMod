use std::fs;

use crate::config::{LOG_FILE, MQTT_CONF};

pub struct MqttConfig {
    pub broker: String,
    pub topic_prefix: String,
    pub username: String,
    pub password: String,
    pub client_id: String,
    pub enabled: bool,
}

impl Default for MqttConfig {
    fn default() -> Self {
        Self {
            broker: "tcp://localhost:1883".to_string(),
            topic_prefix: "taskmod".to_string(),
            username: String::new(),
            password: String::new(),
            client_id: "taskmod-device".to_string(),
            enabled: false,
        }
    }
}

fn parse_mqtt_conf() -> Option<MqttConfig> {
    let content = match fs::read_to_string(MQTT_CONF) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let mut config = MqttConfig::default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            match key.as_str() {
                "enabled" => config.enabled = value == "true" || value == "1",
                "broker" => config.broker = value,
                "topic_prefix" => config.topic_prefix = value,
                "username" => config.username = value,
                "password" => config.password = value,
                "client_id" => config.client_id = value,
                _ => {}
            }
        }
    }

    if !config.enabled {
        return None;
    }
    Some(config)
}

pub fn get_mqtt_config() -> Option<MqttConfig> {
    parse_mqtt_conf()
}

pub fn save_mqtt_config(config: &MqttConfig) -> Result<(), std::io::Error> {
    let content = format!(
        "# TaskMod MQTT配置\n# enabled=true 启用MQTT功能\n# 不配置或enabled=false则不加载MQTT，零内存占用\n\nenabled={}\nbroker={}\ntopic_prefix={}\nusername={}\npassword={}\nclient_id={}",
        if config.enabled { "true" } else { "false" },
        config.broker,
        config.topic_prefix,
        config.username,
        config.password,
        config.client_id
    );
    if let Some(parent) = std::path::Path::new(MQTT_CONF).parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(MQTT_CONF, content)
}

fn log_to_file(msg: &str) {
    let now = chrono::Local::now();
    let log_msg = format!("[{}] [MQTT] {}", now.format("%Y-%m-%d %H:%M:%S"), msg);
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
        .and_then(|mut f| {
            use std::io::Write;
            writeln!(f, "{}", log_msg)
        });
}

// ==================== MQTT 启用时的完整实现 ====================
#[cfg(feature = "mqtt")]
mod mqtt_impl {
    use rumqttc::{AsyncClient, Event, MqttOptions, QoS};
    use serde_json::json;
    use std::sync::{Arc, Mutex};
    use tokio::process::Command;

    use super::{parse_mqtt_conf, log_to_file};

    async fn get_device_status() -> serde_json::Value {
        let mut status = json!({});

        if let Ok(o) = Command::new("getprop").arg("ro.product.model").output().await {
            status["device_model"] = json!(String::from_utf8_lossy(&o.stdout).trim());
        }

        if let Ok(o) = Command::new("getprop").arg("ro.build.version.release").output().await {
            status["android_version"] = json!(String::from_utf8_lossy(&o.stdout).trim());
        }

        if let Ok(content) = fs::read_to_string("/sys/class/power_supply/battery/capacity") {
            status["battery_capacity"] = json!(content.trim());
        } else if let Ok(content) = fs::read_to_string("/sys/class/power_supply/battery0/capacity") {
            status["battery_capacity"] = json!(content.trim());
        }

        if let Ok(content) = fs::read_to_string("/sys/class/power_supply/battery/temp") {
            if let Ok(t) = content.trim().parse::<i32>() {
                status["battery_temperature"] = json!(format!("{:.1}", t as f64 / 10.0));
            }
        } else if let Ok(content) = fs::read_to_string("/sys/class/power_supply/battery0/temp") {
            if let Ok(t) = content.trim().parse::<i32>() {
                status["battery_temperature"] = json!(format!("{:.1}", t as f64 / 10.0));
            }
        }

        if let Ok(content) = fs::read_to_string("/sys/class/power_supply/battery/status") {
            status["battery_status"] = json!(content.trim());
        } else if let Ok(content) = fs::read_to_string("/sys/class/power_supply/battery0/status") {
            status["battery_status"] = json!(content.trim());
        }

        if let Ok(o) = Command::new("uptime").output().await {
            status["uptime"] = json!(String::from_utf8_lossy(&o.stdout).trim());
        }

        if let Ok(o) = Command::new("wm").arg("size").output().await {
            status["screen_size"] = json!(String::from_utf8_lossy(&o.stdout).trim());
        }

        status
    }

    async fn execute_command(cmd: &str) -> String {
        match Command::new("sh").arg("-c").arg(cmd).output().await {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let stderr = String::from_utf8_lossy(&o.stderr);
                if stderr.is_empty() {
                    stdout.to_string()
                } else {
                    format!("{}\nstderr: {}", stdout, stderr)
                }
            }
            Err(e) => format!("执行失败: {}", e),
        }
    }

    static MQTT_CLIENT: Mutex<Option<Arc<AsyncClient>>> = Mutex::new(None);

    pub async fn start_mqtt() {
        let config = match parse_mqtt_conf() {
            Some(c) => c,
            None => {
                log_to_file("MQTT配置未启用或不存在，跳过MQTT启动");
                return;
            }
        };

        log_to_file(&format!("MQTT启动: {}", config.broker));

        let addr = config.broker.trim_start_matches("tcp://").trim_start_matches("ssl://");
        let (host, port) = if let Some((h, p)) = addr.split_once(':') {
            (h, p.parse::<u16>().unwrap_or(1883))
        } else {
            (addr, 1883)
        };

        let mut mqttoptions = MqttOptions::new(&config.client_id, host, port);
        mqttoptions.set_keep_alive(std::time::Duration::from_secs(30));
        mqttoptions.set_clean_session(true);

        if !config.username.is_empty() {
            mqttoptions.set_credentials(&config.username, &config.password);
        }

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        let client_clone = Arc::new(client.clone());
        {
            let mut lock = MQTT_CLIENT.lock().unwrap();
            *lock = Some(client_clone.clone());
        }

        let status_topic = format!("{}/status", config.topic_prefix);
        let cmd_topic = format!("{}/cmd", config.topic_prefix);

        let _ = client.publish(&status_topic, QoS::AtLeastOnce, true, "online").await;
        let _ = client.subscribe(&cmd_topic, QoS::AtLeastOnce).await;

        let client_clone2 = client_clone.clone();
        let prefix_clone = config.topic_prefix.clone();

        tokio::spawn(async move {
            while let Ok(event) = eventloop.poll().await {
                match event {
                    Event::Incoming(packet) => {
                        if let rumqttc::Packet::Publish(publish) = packet {
                            let topic = publish.topic.as_str();
                            let payload = String::from_utf8_lossy(&publish.payload);

                            if topic == cmd_topic {
                                log_to_file(&format!("MQTT收到命令: {}", payload));
                                let result = execute_command(&payload).await;
                                log_to_file(&format!("命令执行结果: {}", result));

                                let reply_topic = format!("{}/result", prefix_clone);
                                let _ = client_clone2.publish(&reply_topic, QoS::AtLeastOnce, false, result).await;
                            }
                        }
                    }
                    Event::Outgoing(_) => {}
                }
            }
        });

        let client_clone3 = client_clone.clone();
        tokio::spawn(async move {
            loop {
                let status = get_device_status().await;
                let payload = serde_json::to_string(&status).unwrap_or_default();
                let _ = client_clone3.publish(&status_topic, QoS::AtLeastOnce, true, payload).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
            }
        });
    }

    pub fn stop_mqtt() {
        if let Some(client) = MQTT_CLIENT.lock().unwrap().take() {
            let status_topic = match parse_mqtt_conf() {
                Some(c) => format!("{}/status", c.topic_prefix),
                None => "taskmod/status".to_string(),
            };
            let _ = tokio::runtime::Handle::current().block_on(client.publish(&status_topic, QoS::AtLeastOnce, true, "offline"));
            log_to_file("MQTT服务已停止");
        }
    }

    pub async fn publish(topic: &str, payload: String) -> Result<(), String> {
        let client = match MQTT_CLIENT.lock().unwrap().as_ref() {
            Some(c) => c.clone(),
            None => return Err("MQTT客户端未连接".to_string()),
        };

        let _ = client.publish(topic, QoS::AtLeastOnce, false, payload).await;
        Ok(())
    }
}

// ==================== MQTT 禁用时的桩实现 ====================
#[cfg(not(feature = "mqtt"))]
mod mqtt_impl {
    pub async fn start_mqtt() {
        super::log_to_file("MQTT功能已禁用（编译时未启用mqtt feature）");
    }

    pub fn stop_mqtt() {}

    pub async fn publish(_topic: &str, _payload: String) -> Result<(), String> {
        Err("MQTT功能已禁用".to_string())
    }
}

pub use mqtt_impl::{start_mqtt, stop_mqtt, publish};
