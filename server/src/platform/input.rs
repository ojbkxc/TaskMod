use async_trait::async_trait;

/// 跨平台输入控制 trait
#[async_trait]
pub trait InputController: Send {
    /// 点击屏幕指定位置
    async fn tap(&self, x: i32, y: i32) -> Result<(), String>;
    /// 滑动操作
    async fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u64) -> Result<(), String>;
    /// 按键事件
    async fn key_event(&self, keycode: i32) -> Result<(), String>;
    /// 输入文本
    async fn input_text(&self, text: &str) -> Result<(), String>;
    /// 获取剪贴板内容
    async fn get_clipboard(&self) -> Result<String, String>;
    /// 设置剪贴板内容
    async fn set_clipboard(&self, text: &str) -> Result<(), String>;
}

// ==================== Android 实现 ====================

#[cfg(any(target_os = "android", target_os = "linux"))]
pub struct AndroidInput;

#[cfg(any(target_os = "android", target_os = "linux"))]
impl AndroidInput {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(any(target_os = "android", target_os = "linux"))]
#[async_trait]
impl InputController for AndroidInput {
    async fn tap(&self, x: i32, y: i32) -> Result<(), String> {
        run_cmd("input", &["tap", &x.to_string(), &y.to_string()]).await
    }

    async fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u64) -> Result<(), String> {
        run_cmd("input", &[
            "swipe",
            &x1.to_string(), &y1.to_string(),
            &x2.to_string(), &y2.to_string(),
            &duration_ms.to_string(),
        ]).await
    }

    async fn key_event(&self, keycode: i32) -> Result<(), String> {
        run_cmd("input", &["keyevent", &keycode.to_string()]).await
    }

    async fn input_text(&self, text: &str) -> Result<(), String> {
        // Android input text 中空格需要用引号包裹，或用 %s 但需要先加双引号
        // 最可靠的方式: 用单引号包裹整个文本，内部单号用 '\''' 转义
        let escaped = text.replace('\'', "'\\''");
        run_cmd("input", &["text", &format!("'{}'", escaped)]).await
    }

    async fn get_clipboard(&self) -> Result<String, String> {
        let output = tokio::process::Command::new("cmd")
            .args(["clipboard", "get"])
            .output()
            .await
            .map_err(|e| format!("获取剪贴板失败: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            // Fallback: 使用 service call clipboard
            let output = tokio::process::Command::new("service")
                .args(["call", "clipboard", "2"])
                .output()
                .await
                .map_err(|e| format!("获取剪贴板失败: {}", e))?;
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        }
    }

    async fn set_clipboard(&self, text: &str) -> Result<(), String> {
        run_cmd("cmd", &["clipboard", "set", text]).await
    }
}

// ==================== Windows 实现 ====================

#[cfg(target_os = "windows")]
pub struct WindowsInput;

#[cfg(target_os = "windows")]
impl WindowsInput {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "windows")]
#[async_trait]
impl InputController for WindowsInput {
    async fn tap(&self, x: i32, y: i32) -> Result<(), String> {
        // 使用 PowerShell 的 System.Windows.Forms.Cursor 和 SendInput
        let script = format!(
            r#"
            Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class MouseInput {{
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int X, int Y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);

    public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    public const uint MOUSEEVENTF_LEFTUP = 0x0004;

    public static void Click(int x, int y) {{
        SetCursorPos(x, y);
        mouse_event(MOUSEEVENTF_LEFTDOWN, x, y, 0, IntPtr.Zero);
        mouse_event(MOUSEEVENTF_LEFTUP, x, y, 0, IntPtr.Zero);
    }}
}}
"@
            [MouseInput]::Click({}, {})
            "#,
            x, y
        );
        run_powershell(&script).await
    }

    async fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u64) -> Result<(), String> {
        let steps = (duration_ms / 16).max(1); // ~60fps
        let dx = (x2 - x1) as f64 / steps as f64;
        let dy = (y2 - y1) as f64 / steps as f64;

        let script = format!(
            r#"
            Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class MouseSwipe {{
    [DllImport("user32.dll")]
    public static extern bool SetCursorPos(int X, int Y);

    [DllImport("user32.dll")]
    public static extern void mouse_event(uint dwFlags, int dx, int dy, uint dwData, IntPtr dwExtraInfo);

    public const uint MOUSEEVENTF_LEFTDOWN = 0x0002;
    public const uint MOUSEEVENTF_LEFTUP = 0x0004;
    public const uint MOUSEEVENTF_MOVE = 0x0001;
}}
"@
            [MouseSwipe]::SetCursorPos({}, {})
            [MouseSwipe]::mouse_event([MouseSwipe]::MOUSEEVENTF_LEFTDOWN, 0, 0, 0, [IntPtr]::Zero)
            for ($i = 1; $i -le {}; $i++) {{
                $cx = [int]({} + {} * $i)
                $cy = [int]({} + {} * $i)
                [MouseSwipe]::SetCursorPos($cx, $cy)
                Start-Sleep -Milliseconds 16
            }}
            [MouseSwipe]::mouse_event([MouseSwipe]::MOUSEEVENTF_LEFTUP, 0, 0, 0, [IntPtr]::Zero)
            "#,
            x1, y1, steps, x1 as f64, dx, y1 as f64, dy
        );
        run_powershell(&script).await
    }

    async fn key_event(&self, keycode: i32) -> Result<(), String> {
        // 将 Android keycode 映射到 Windows VK
        let vk = android_keycode_to_windows_vk(keycode);
        let script = format!(
            r#"
            Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;

public class KeyInput {{
    [DllImport("user32.dll")]
    public static extern void key_event(byte bVk, byte bScan, uint dwFlags, IntPtr dwExtraInfo);

    public const uint KEYEVENTF_KEYDOWN = 0x0000;
    public const uint KEYEVENTF_KEYUP = 0x0002;
}}
"@
            [KeyInput]::key_event({}, 0, [KeyInput]::KEYEVENTF_KEYDOWN, [IntPtr]::Zero)
            [KeyInput]::key_event({}, 0, [KeyInput]::KEYEVENTF_KEYUP, [IntPtr]::Zero)
            "#,
            vk, vk
        );
        run_powershell(&script).await
    }

    async fn input_text(&self, text: &str) -> Result<(), String> {
        // 使用 PowerShell SendKeys
        let escaped = text.replace('{', "{{").replace('}', "}}")
            .replace('+', "{+}").replace('^', "{^}")
            .replace('%', "{%}").replace('~', "{~}")
            .replace('(', "{(}").replace(')', "{)}");
        let script = format!(
            r#"Add-Type -AssemblyName System.Windows.Forms; [System.Windows.Forms.SendKeys]::SendWait("{}")"#,
            escaped.replace('"', "`\"")
        );
        run_powershell(&script).await
    }

    async fn get_clipboard(&self) -> Result<String, String> {
        let output = tokio::process::Command::new("powershell")
            .args(["-Command", "Get-Clipboard"])
            .output()
            .await
            .map_err(|e| format!("获取剪贴板失败: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err("获取剪贴板失败".to_string())
        }
    }

    async fn set_clipboard(&self, text: &str) -> Result<(), String> {
        let script = format!("Set-Clipboard -Value '{}'", text.replace('\'', "''"));
        run_powershell(&script).await
    }
}

// ==================== Linux 实现 ====================

#[cfg(target_os = "linux")]
pub struct LinuxInput;

#[cfg(target_os = "linux")]
impl LinuxInput {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_os = "linux")]
#[async_trait]
impl InputController for LinuxInput {
    async fn tap(&self, x: i32, y: i32) -> Result<(), String> {
        run_cmd("xdotool", &["mousemove", &x.to_string(), &y.to_string()]).await?;
        run_cmd("xdotool", &["click", "1"]).await
    }

    async fn swipe(&self, x1: i32, y1: i32, x2: i32, y2: i32, duration_ms: u64) -> Result<(), String> {
        run_cmd("xdotool", &["mousemove", &x1.to_string(), &y1.to_string()]).await?;
        run_cmd("xdotool", &["mousedown", "1"]).await?;
        // 使用 xdotool 的 mousemove --delay 实现滑动
        let steps = (duration_ms / 50).max(1);
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            let cx = x1 + ((x2 - x1) as f64 * t) as i32;
            let cy = y1 + ((y2 - y1) as f64 * t) as i32;
            run_cmd("xdotool", &["mousemove", &cx.to_string(), &cy.to_string()]).await?;
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        run_cmd("xdotool", &["mouseup", "1"]).await
    }

    async fn key_event(&self, keycode: i32) -> Result<(), String> {
        let xk = android_keycode_to_linux_xk(keycode);
        run_cmd("xdotool", &["key", xk]).await
    }

    async fn input_text(&self, text: &str) -> Result<(), String> {
        run_cmd("xdotool", &["type", "--delay", "0", text]).await
    }

    async fn get_clipboard(&self) -> Result<String, String> {
        let output = tokio::process::Command::new("xclip")
            .args(["-selection", "clipboard", "-o"])
            .output()
            .await
            .map_err(|e| format!("获取剪贴板失败: {}", e))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err("获取剪贴板失败（需要安装 xclip）".to_string())
        }
    }

    async fn set_clipboard(&self, text: &str) -> Result<(), String> {
        let mut child = tokio::process::Command::new("xclip")
            .args(["-selection", "clipboard"])
            .stdin(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("设置剪贴板失败: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(text.as_bytes()).await.map_err(|e| e.to_string())?;
        }

        child.wait().await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

// ==================== 工厂函数 ====================

/// 创建跨平台输入控制器
pub fn create_input_controller() -> Box<dyn InputController> {
    let platform = crate::platform::detect_platform();

    match platform {
        crate::platform::Platform::Android => {
            #[cfg(any(target_os = "android", target_os = "linux"))]
            { Box::new(AndroidInput::new()) }
            #[cfg(not(any(target_os = "android", target_os = "linux")))]
            { Box::new(NullInput) }
        }
        crate::platform::Platform::Windows => {
            #[cfg(target_os = "windows")]
            { Box::new(WindowsInput::new()) }
            #[cfg(not(target_os = "windows"))]
            { Box::new(NullInput) }
        }
        crate::platform::Platform::Linux => {
            #[cfg(target_os = "linux")]
            { Box::new(LinuxInput::new()) }
            #[cfg(not(target_os = "linux"))]
            { Box::new(NullInput) }
        }
        _ => Box::new(NullInput),
    }
}

/// 空实现（不支持的平台）
struct NullInput;

#[async_trait]
impl InputController for NullInput {
    async fn tap(&self, _: i32, _: i32) -> Result<(), String> { Err("平台不支持".to_string()) }
    async fn swipe(&self, _: i32, _: i32, _: i32, _: i32, _: u64) -> Result<(), String> { Err("平台不支持".to_string()) }
    async fn key_event(&self, _: i32) -> Result<(), String> { Err("平台不支持".to_string()) }
    async fn input_text(&self, _: &str) -> Result<(), String> { Err("平台不支持".to_string()) }
    async fn get_clipboard(&self) -> Result<String, String> { Err("平台不支持".to_string()) }
    async fn set_clipboard(&self, _: &str) -> Result<(), String> { Err("平台不支持".to_string()) }
}

// ==================== 工具函数 ====================

#[cfg(any(target_os = "android", target_os = "linux"))]
async fn run_cmd(cmd: &str, args: &[&str]) -> Result<(), String> {
    let output = tokio::process::Command::new(cmd)
        .args(args)
        .output()
        .await
        .map_err(|e| format!("命令执行失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("命令失败: {}", err))
    }
}

#[cfg(target_os = "windows")]
async fn run_powershell(script: &str) -> Result<(), String> {
    let output = tokio::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .output()
        .await
        .map_err(|e| format!("PowerShell 执行失败: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        Err(format!("PowerShell 失败: {}", err))
    }
}

/// Android keycode 到 Windows VK 码映射
#[cfg(target_os = "windows")]
fn android_keycode_to_windows_vk(keycode: i32) -> u8 {
    match keycode {
        3 => 0x5B,     // HOME -> VK_LWIN
        4 => 0xA4,     // BACK -> VK_LMENU (Alt)
        24 => 0xAF,    // VOLUME_UP -> VK_VOLUME_UP
        25 => 0xAE,    // VOLUME_DOWN -> VK_VOLUME_DOWN
        26 => 0x73,    // POWER -> VK_F4 (关机)
        66 => 0x0D,    // ENTER -> VK_RETURN
        67 => 0x08,    // DEL -> VK_BACK
        111 => 0x1B,   // ESCAPE -> VK_ESCAPE
        112 => 0x2E,   // FORWARD_DEL -> VK_DELETE
        187 => 0x5B,   // APP_SWITCH -> VK_LWIN
        19 => 0x26,    // DPAD_UP -> VK_UP
        20 => 0x28,    // DPAD_DOWN -> VK_DOWN
        21 => 0x25,    // DPAD_LEFT -> VK_LEFT
        22 => 0x27,    // DPAD_RIGHT -> VK_RIGHT
        23 => 0x0D,    // DPAD_CENTER -> VK_RETURN
        _ => keycode as u8,
    }
}

/// Android keycode 到 Linux XK keysym 映射
#[cfg(target_os = "linux")]
fn android_keycode_to_linux_xk(keycode: i32) -> &'static str {
    match keycode {
        3 => "Super_L",     // HOME
        4 => "Escape",      // BACK
        24 => "XF86AudioRaiseVolume", // VOLUME_UP
        25 => "XF86AudioLowerVolume", // VOLUME_DOWN
        26 => "XF86Power",  // POWER
        66 => "Return",     // ENTER
        67 => "BackSpace",  // DEL
        111 => "Escape",    // ESCAPE
        112 => "Delete",    // FORWARD_DEL
        187 => "Super_L",   // APP_SWITCH
        19 => "Up",         // DPAD_UP
        20 => "Down",       // DPAD_DOWN
        21 => "Left",       // DPAD_LEFT
        22 => "Right",      // DPAD_RIGHT
        23 => "Return",     // DPAD_CENTER
        _ => "Return",
    }
}
