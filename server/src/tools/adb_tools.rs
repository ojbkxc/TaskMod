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
    fn description(&self) -> &str {
        "模拟点击屏幕指定坐标位置。使用前请先用get_device_info获取屏幕分辨率，确保坐标在有效范围内。\
         坐标系: 左上角(0,0)，X轴向右增大，Y轴向下增大。\
         常见1080x2400屏幕: 状态栏约y=100，底部导航栏约y=2300。"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "x": {"type": "integer", "description": "X坐标（像素），从左到右，通常范围 0~1080"},
                "y": {"type": "integer", "description": "Y坐标（像素），从上到下，通常范围 0~2400"}
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
    fn description(&self) -> &str {
        "模拟屏幕滑动操作，从起始坐标滑动到结束坐标。\
         用于: 下拉通知栏(y1小->y2大)、上滑返回(y1大->y2小)、左右翻页等。\
         注意: 滑动距离建议>=200像素，否则可能被识别为点击而非滑动。"
    }
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
    fn description(&self) -> &str {
        "模拟物理按键事件。\
         常用按键: back(返回)、home(主屏幕)、power(电源键/唤醒屏幕)、\
         volume_up(音量加)、volume_down(音量减)、recents(最近任务)、\
         enter(回车)、delete(删除)、menu(菜单)、\
         page_up(上翻页)、page_down(下翻页)、escape(返回/关闭)。\
         唤醒屏幕: 先用power唤醒，再用adb_unlock_screen解锁。"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "key": {
                    "type": "string",
                    "description": "按键名称",
                    "enum": ["back", "home", "power", "volume_up", "volume_down", "recents", "menu", "enter", "delete", "tab", "space", "camera", "search", "page_up", "page_down", "escape"]
                }
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
    fn description(&self) -> &str {
        "向当前聚焦的输入框输入文本。注意: 调用前需确保输入框已获得焦点（可先用adb_tap点击输入框）。\
         只支持ASCII字符和中文，不支持直接输入换行符。如果输入失败，可以尝试用adb_command执行 'input text \"文本\"'。"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "text": {"type": "string", "description": "要输入的文本内容（仅ASCII和中文，空格会被自动处理）"}
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
    fn description(&self) -> &str {
        "截取当前屏幕画面并保存为PNG文件。文件保存在 /sdcard/TaskMod/screenshots/ 目录下。"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "filename": {"type": "string", "description": "保存的文件名（含路径），如 /sdcard/TaskMod/screenshots/test.png"}
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
    fn description(&self) -> &str {
        "执行任意shell命令（通过 /system/bin/sh -c），支持管道、重定向等复杂语法。\
         当专用工具不够用时可用此工具。例如: 'ls /sdcard/', 'pm list packages', 'dumpsys activity tops'。\
         危险操作需先确认用户意图。"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "command": {"type": "string", "description": "要执行的shell命令字符串"}
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
    fn description(&self) -> &str {
        "启动指定应用。需传入应用包名(package name)，如 com.android.settings(设置)、\
         com.tencent.mm(微信)、com.android.chrome(浏览器)。\
         如果不确定包名，先用get_running_apps查看或用adb_command执行'pm list packages -3'查看第三方应用。"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "package_name": {"type": "string", "description": "应用包名，如 com.android.settings、com.tencent.mm"}
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
    fn description(&self) -> &str {
        "强制停止指定应用（am force-stop），等同于从任务管理器中结束应用。"
    }
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
    fn description(&self) -> &str {
        "获取当前WiFi连接信息，包括SSID(网络名称)、BSSID(路由器MAC)、IP地址。"
    }
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
    fn description(&self) -> &str {
        "获取设备系统信息，包括设备型号、Android版本、存储空间。建议在执行屏幕操作前先调用此工具获取屏幕分辨率。"
    }
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
    fn description(&self) -> &str {
        "获取电池详细信息，包括电量百分比、充电状态、温度、健康状态等。"
    }
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
    fn description(&self) -> &str {
        "获取当前正在运行的应用列表（最多显示20个第三方应用）。可用于确认某个应用是否在运行。"
    }
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
    fn description(&self) -> &str {
        "重启设备。这是一个危险操作，执行前务必确认用户意图！"
    }
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
    fn description(&self) -> &str {
        "关闭设备（需要root权限）。这是一个危险操作，执行前务必确认用户意图！"
    }
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
    fn description(&self) -> &str {
        "清除指定应用的所有数据（缓存、用户数据、数据库等），等同于在设置中「清除数据」。需要root权限。此操作不可逆，执行前务必确认用户意图！"
    }
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
    fn description(&self) -> &str {
        "使用系统TTS语音引擎播放文本语音。会依次尝试广播、cmd speech、am startservice三种方式。\
         如果播放失败可能是因为设备未安装TTS引擎。"
    }
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
    fn description(&self) -> &str {
        "唤醒屏幕并上滑解锁。会自动获取屏幕分辨率计算滑动坐标。\
         仅适用于无密码/图案锁屏的设备（滑动锁屏）。如果有密码锁屏，此操作只能唤醒屏幕并上滑，但无法解锁。"
    }
    fn parameters(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }
    fn execute(&self, _args: &str) -> BoxFuture<'_, String> {
        Box::pin(async move {
            // 先唤醒屏幕，再上滑解锁
            let _ = adb::keyevent("224").await;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
            // 动态获取屏幕分辨率，按比例计算滑动坐标
            let size_str = adb::get_screen_size().await;
            let parts: Vec<&str> = size_str.split('x').collect();
            let (w, h) = if parts.len() == 2 {
                (parts[0].parse::<i32>().unwrap_or(1080), parts[1].parse::<i32>().unwrap_or(1920))
            } else {
                (1080, 1920)
            };
            let x = w / 2;
            let y_start = h * 80 / 100;  // 底部 80%
            let y_end = h * 30 / 100;    // 滑到 30%
            adb::swipe(x, y_start, x, y_end).await
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
