use futures::future::BoxFuture;
use crate::tools::{AiTool, parse_arg};
use crate::utils::adb;
use serde_json::json;

pub struct AdbTapTool;
pub struct AdbSwipeTool;
pub struct AdbKeyeventTool;
pub struct AdbInputTextTool;
pub struct AdbScreencapTool;
pub struct AdbCommandTool;
pub struct AdbStartAppTool;
pub struct AdbStopAppTool;
pub struct GetWifiInfoTool;
pub struct GetDeviceInfoTool;
pub struct GetBatteryInfoTool;
pub struct GetRunningAppsTool;
pub struct AdbRebootTool;
pub struct AdbShutdownTool;
pub struct AdbClearAppDataTool;
pub struct AdbTtsTool;

impl AiTool for AdbTapTool {
    fn name(&self) -> &str { "adb_tap" }
    fn description(&self) -> &str { "点击屏幕指定位置" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "x": {"type": "integer", "description": "X坐标"},
                "y": {"type": "integer", "description": "Y坐标"}
            },
            "required": ["x", "y"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match (parse_arg::<i32>(&args, "x"), parse_arg::<i32>(&args, "y")) {
                (Ok(x), Ok(y)) => adb::tap(x, y).await,
                (Err(e), _) | (_, Err(e)) => e,
            }
        })
    }
}

impl AiTool for AdbSwipeTool {
    fn name(&self) -> &str { "adb_swipe" }
    fn description(&self) -> &str { "滑动屏幕" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "x1": {"type": "integer", "description": "起始X坐标"},
                "y1": {"type": "integer", "description": "起始Y坐标"},
                "x2": {"type": "integer", "description": "结束X坐标"},
                "y2": {"type": "integer", "description": "结束Y坐标"}
            },
            "required": ["x1", "y1", "x2", "y2"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match (parse_arg::<i32>(&args, "x1"), parse_arg::<i32>(&args, "y1"),
                   parse_arg::<i32>(&args, "x2"), parse_arg::<i32>(&args, "y2")) {
                (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) => adb::swipe(x1, y1, x2, y2).await,
                _ => "参数解析失败".to_string(),
            }
        })
    }
}

impl AiTool for AdbKeyeventTool {
    fn name(&self) -> &str { "adb_keyevent" }
    fn description(&self) -> &str { "模拟按键事件" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "key": {"type": "string", "description": "按键名称: back, home, power, volume_up, volume_down, recents"}
            },
            "required": ["key"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "key") {
                Ok(key) => adb::keyevent(&key).await,
                Err(e) => e,
            }
        })
    }
}

impl AiTool for AdbInputTextTool {
    fn name(&self) -> &str { "adb_input_text" }
    fn description(&self) -> &str { "输入文本" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string", "description": "要输入的文本"}
            },
            "required": ["text"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "text") {
                Ok(text) => adb::input_text(&text).await,
                Err(e) => e,
            }
        })
    }
}

impl AiTool for AdbScreencapTool {
    fn name(&self) -> &str { "adb_screencap" }
    fn description(&self) -> &str { "截取屏幕并保存" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "filename": {"type": "string", "description": "保存的文件名"}
            },
            "required": ["filename"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "filename") {
                Ok(filename) => adb::screencap(&filename).await,
                Err(e) => e,
            }
        })
    }
}

impl AiTool for AdbCommandTool {
    fn name(&self) -> &str { "adb_command" }
    fn description(&self) -> &str { "执行任意ADB命令" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "要执行的命令"}
            },
            "required": ["command"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "command") {
                Ok(command) => adb::run_command(&command).await.unwrap_or_else(|e| e),
                Err(e) => e,
            }
        })
    }
}

impl AiTool for AdbStartAppTool {
    fn name(&self) -> &str { "adb_start_app" }
    fn description(&self) -> &str { "启动指定应用" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "package_name": {"type": "string", "description": "应用包名，如 com.android.settings"}
            },
            "required": ["package_name"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "package_name") {
                Ok(name) => adb::start_app(&name).await,
                Err(e) => e,
            }
        })
    }
}

impl AiTool for AdbStopAppTool {
    fn name(&self) -> &str { "adb_stop_app" }
    fn description(&self) -> &str { "强制停止指定应用" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "package_name": {"type": "string", "description": "应用包名，如 com.android.settings"}
            },
            "required": ["package_name"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "package_name") {
                Ok(name) => adb::stop_app(&name).await,
                Err(e) => e,
            }
        })
    }
}

impl AiTool for GetWifiInfoTool {
    fn name(&self) -> &str { "get_wifi_info" }
    fn description(&self) -> &str { "获取当前WiFi连接信息" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            adb::get_wifi_info().await
        })
    }
}

impl AiTool for GetDeviceInfoTool {
    fn name(&self) -> &str { "get_device_info" }
    fn description(&self) -> &str { "获取设备系统信息（电池、存储、型号等）" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            adb::get_device_info().await
        })
    }
}

impl AiTool for GetBatteryInfoTool {
    fn name(&self) -> &str { "get_battery_info" }
    fn description(&self) -> &str { "获取电池信息（电量、温度等）" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            adb::get_battery_info().await
        })
    }
}

impl AiTool for GetRunningAppsTool {
    fn name(&self) -> &str { "get_running_apps" }
    fn description(&self) -> &str { "获取当前运行的应用列表" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            adb::get_running_apps().await
        })
    }
}

impl AiTool for AdbRebootTool {
    fn name(&self) -> &str { "adb_reboot" }
    fn description(&self) -> &str { "重启设备" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            adb::reboot().await
        })
    }
}

impl AiTool for AdbShutdownTool {
    fn name(&self) -> &str { "adb_shutdown" }
    fn description(&self) -> &str { "关闭设备（需要root权限）" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            adb::shutdown().await
        })
    }
}

impl AiTool for AdbClearAppDataTool {
    fn name(&self) -> &str { "adb_clear_app_data" }
    fn description(&self) -> &str { "清除指定应用的数据（需要root权限）" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "package_name": {"type": "string", "description": "应用包名"}
            },
            "required": ["package_name"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "package_name") {
                Ok(name) => adb::clear_app_data(&name).await,
                Err(e) => e,
            }
        })
    }
}

impl AiTool for AdbTtsTool {
    fn name(&self) -> &str { "adb_tts" }
    fn description(&self) -> &str { "使用系统TTS语音播放文本（支持小爱同学等语音助手）" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string", "description": "要播放的文本内容"}
            },
            "required": ["text"]
        })
    }
    fn execute(&self, args: &str) -> BoxFuture<'_, String> {
        let args = args.to_string();
        Box::pin(async move {
            match parse_arg::<String>(&args, "text") {
                Ok(text) => adb::tts(&text).await,
                Err(e) => e,
            }
        })
    }
}

pub struct AdbUnlockTool;

impl AiTool for AdbUnlockTool {
    fn name(&self) -> &str { "adb_unlock_screen" }
    fn description(&self) -> &str { "上滑解锁屏幕（从屏幕底部向上滑动）" }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            // 先唤醒屏幕，再上滑解锁
            let _ = adb::keyevent("wakeup").await;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            // 从屏幕底部 80% 处滑到 30% 处，适配大多数手机
            adb::swipe(540, 1800, 540, 600).await
        })
    }
}

#[allow(dead_code)]
pub fn register_adb_tools(registry: &mut crate::tools::ToolRegistry) {
    registry.register(Box::new(AdbTapTool));
    registry.register(Box::new(AdbSwipeTool));
    registry.register(Box::new(AdbKeyeventTool));
    registry.register(Box::new(AdbInputTextTool));
    registry.register(Box::new(AdbScreencapTool));
    registry.register(Box::new(AdbCommandTool));
    registry.register(Box::new(AdbStartAppTool));
    registry.register(Box::new(AdbStopAppTool));
    registry.register(Box::new(GetWifiInfoTool));
    registry.register(Box::new(GetDeviceInfoTool));
    registry.register(Box::new(GetBatteryInfoTool));
    registry.register(Box::new(GetRunningAppsTool));
    registry.register(Box::new(AdbRebootTool));
    registry.register(Box::new(AdbShutdownTool));
    registry.register(Box::new(AdbClearAppDataTool));
    registry.register(Box::new(AdbTtsTool));
    registry.register(Box::new(AdbUnlockTool));
}