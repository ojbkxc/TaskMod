use dioxus::prelude::*;
use eq_ui::prelude::*;
use wasm_bindgen::closure::Closure;
use web_sys::MessageEvent;
use crate::api::client::{execute_command, get_device_info, DeviceInfo};

#[component]
pub fn MirrorPage() -> Element {
    let mut is_connected = use_signal(|| false);
    let mut audio_enabled = use_signal(|| false);
    let device_info = use_signal(|| Option::<DeviceInfo>::None);
    let is_refreshing = use_signal(|| false);
    let cmd_output = use_signal(|| "等待执行命令...".to_string());

    let load_device_info = move || {
        is_refreshing.set(true);
        spawn(async move {
            match get_device_info().await {
                Ok(info) => {
                    device_info.set(Some(info));
                }
                Err(_) => {
                    device_info.set(None);
                }
            }
            is_refreshing.set(false);
        });
    };

    use_effect(move || {
        load_device_info();
    });

    let exec_adb = move |label: &'static str| {
        let cmd: Option<String> = match label {
            "唤醒" => Some("input keyevent 224".to_string()),
            "息屏" => Some("input keyevent 26".to_string()),
            "解锁" => Some("input keyevent 82; input swipe 540 1800 540 600 300".to_string()),
            "Home" => Some("input keyevent 3".to_string()),
            "返回" => Some("input keyevent 4".to_string()),
            "重启设备" => Some("reboot".to_string()),
            "关闭设备" => Some("reboot -p".to_string()),
            "启动应用" => {
                let window = web_sys::window().expect("window should be available");
                match window.prompt_with_message("请输入要启动的应用包名 (如 com.example.app):") {
                    Ok(Some(pkg)) if !pkg.trim().is_empty() => {
                        Some(format!("monkey -p {} -c android.intent.category.LAUNCHER 1", pkg.trim()))
                    }
                    _ => {
                        cmd_output.set("[启动应用] 已取消或包名为空".to_string());
                        return;
                    }
                }
            }
            "停止应用" => {
                let window = web_sys::window().expect("window should be available");
                match window.prompt_with_message("请输入要停止的应用包名 (如 com.example.app):") {
                    Ok(Some(pkg)) if !pkg.trim().is_empty() => {
                        Some(format!("am force-stop {}", pkg.trim()))
                    }
                    _ => {
                        cmd_output.set("[停止应用] 已取消或包名为空".to_string());
                        return;
                    }
                }
            }
            _ => None,
        };
        if let Some(cmd) = cmd {
            cmd_output.set(format!("正在执行: {} ...", cmd));
            spawn(async move {
                match execute_command(&cmd).await {
                    Ok(result) => {
                        cmd_output.set(format!("[{}] 执行成功:\n{}", label, result));
                    }
                    Err(e) => {
                        cmd_output.set(format!("[{}] 执行失败: {}", label, e));
                    }
                }
            });
        }
    };

    let start_mirror = move |_| {
        is_connected.set(true);
        // 启动音频播放
        audio_enabled.set(true);
        spawn(async move {
            use web_sys::WebSocket;
            use wasm_bindgen::JsCast;
            use js_sys::Uint8Array;

            let ws = match WebSocket::new("/ws/mirror/audio") {
                Ok(ws) => ws,
                Err(_) => return,
            };
            ws.set_binary_type(web_sys::BinaryType::Arraybuffer);

            let audio_ctx = web_sys::AudioContext::new().ok();
            let sample_rate = 48000.0;

            let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |ev| {
                if let Some(ab) = ev.data().dyn_ref::<js_sys::ArrayBuffer>() {
                    if let Some(ctx) = &audio_ctx {
                        let array = Uint8Array::new(&ab);
                        let len = array.length() as usize / 2;
                        let mut samples = Vec::with_capacity(len);
                        let bytes = array.to_vec();
                        for i in (0..bytes.len()).step_by(2) {
                            if i + 1 < bytes.len() {
                                let v = i16::from_le_bytes([bytes[i], bytes[i + 1]]);
                                samples.push(v as f32 / 32768.0);
                            }
                        }
                        if samples.is_empty() { return; }
                        if let Ok(buffer) = ctx.create_buffer(1, samples.len() as u32, sample_rate as f32) {
                            buffer.copy_to_channel(&samples, 0);
                            if let Ok(src) = ctx.create_buffer_source() {
                                src.set_buffer(Some(&buffer));
                                let _ = src.connect_with_audio_node(&ctx.destination());
                                let _ = src.start();
                            }
                        }
                    }
                }
            });
            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();
        });
    };

    rsx! {
        div { class: "p-4 space-y-3",
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-2",
                    span {
                        class: "w-2.5 h-2.5 rounded-full",
                        background_color: if *is_connected.read() { "var(--ds-success)" } else { "var(--ds-text-tertiary)" }
                    }
                    span { class: "text-sm text-[var(--ds-text-secondary)]",
                        if *is_connected.read() { "已连接" } else { "未连接" }
                    }
                    if *audio_enabled.read() {
                        span { class: "ml-2 text-[10px] px-1.5 py-0.5 rounded-full bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)]",
                            "音频已开启"
                        }
                    }
                }
                div { class: "flex items-center gap-2",
                    if *is_connected.read() {
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            onclick: move |_| {
                                is_connected.set(false);
                                audio_enabled.set(false);
                            },
                            "停止投屏"
                        }
                    } else {
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            onclick: start_mirror,
                            "开始投屏"
                        }
                    }
                }
            }

            div { class: "hidden md:flex gap-4 mb-4",
                div { class: "flex-1",
                    EqCard { class: "p-4",
                        div { class: "flex items-center justify-between mb-3",
                            div { class: "flex items-center gap-2",
                                svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                                }
                                span { class: "text-sm font-semibold text-[var(--ds-text)]", "ADB 命令" }
                            }
                        }
                        div { class: "flex gap-2 mb-3",
                            input {
                                class: "flex-1 min-h-[36px] px-2.5 py-1.5 border border-[var(--ds-border)] rounded bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] font-mono outline-none focus:border-[var(--ds-blue)]",
                                placeholder: "输入命令...",
                            }
                            EqButton {
                                variant: EqButtonVariant::Primary,
                                "执行"
                            }
                        }
                        div { class: "grid grid-cols-5 gap-1.5",
                            AdbCommandCard { label: "唤醒", onclick: move |_| exec_adb("唤醒") }
                            AdbCommandCard { label: "息屏", onclick: move |_| exec_adb("息屏") }
                            AdbCommandCard { label: "解锁", onclick: move |_| exec_adb("解锁") }
                            AdbCommandCard { label: "Home", onclick: move |_| exec_adb("Home") }
                            AdbCommandCard { label: "返回", onclick: move |_| exec_adb("返回") }
                        }
                    }
                }

                div { class: "w-64",
                    EqCard { class: "p-4",
                        div { class: "flex items-center justify-between mb-3",
                            div { class: "flex items-center gap-2",
                                svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "设备信息" }
                            button {
                                class: "p-1 hover:bg-[var(--ds-surface)] rounded transition-colors",
                                onclick: move |_| load_device_info(),
                                svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                                }
                            }
                        }
                        if *is_refreshing.read() {
                            div { class: "flex items-center justify-center py-4",
                                div { class: "w-4 h-4 border-2 border-[var(--ds-border)] border-t-[var(--ds-blue)] rounded-full animate-spin" }
                            }
                        } else if let Some(info) = device_info.read().as_ref() {
                            div { class: "space-y-2",
                                DeviceInfoItem { label: "型号", value: info.model.clone() }
                                DeviceInfoItem { label: "电量", value: info.battery.clone() }
                                DeviceInfoItem { label: "分辨率", value: info.screen_size.clone() }
                                DeviceInfoItem { label: "WiFi", value: info.wifi.clone() }
                                DeviceInfoItem { label: "IP", value: info.ip.clone() }
                                DeviceInfoItem { label: "Android", value: info.android_version.clone() }
                            }
                        } else {
                            div { class: "text-center py-2",
                                span { class: "text-xs text-[var(--ds-text-tertiary)]", "点击刷新获取设备信息" }
                            }
                        }
                    }
                }
            }

            div { class: "hidden md:flex items-center justify-center min-w-0",
                div { class: "relative border border-[var(--ds-border)] rounded-lg overflow-hidden bg-[#0a0a0f] flex items-center justify-center w-full max-w-[400px] aspect-[9/16] max-h-[78vh] mx-auto shadow-lg",
                    if *is_connected.read() {
                        div { class: "flex flex-col items-center gap-3 p-10 text-center",
                            p { class: "text-sm text-[var(--ds-text-secondary)]", "投屏中..." }
                        }
                    } else {
                        div { class: "flex flex-col items-center gap-3 p-10 text-center",
                            div { class: "w-16 h-16 flex items-center justify-center bg-[var(--ds-surface)] border border-[var(--ds-border)] rounded-2xl text-2xl text-[var(--ds-text-tertiary)] opacity-60",
                                svg { class: "w-7 h-7", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" }
                                }
                            }
                            p { class: "text-sm font-semibold text-[var(--ds-text-secondary)]", "设备未连接" }
                            p { class: "text-xs text-[var(--ds-text-tertiary)]", "点击上方\"开始投屏\"连接设备屏幕" }
                        }
                    }
                }
            }

            div { class: "hidden md:flex gap-4 mt-4",
                div { class: "flex-1",
                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "命令输出" }
                        }
                        div { class: "min-h-[100px] max-h-[200px] overflow-y-auto p-2.5 bg-[var(--ds-bg)] border border-[var(--ds-border)] rounded text-xs font-mono text-[var(--ds-text-secondary)] whitespace-pre-wrap break-all",
                            "{cmd_output.read()}"
                        }
                    }
                }

                div { class: "flex-1",
                    div { class: "grid grid-cols-2 gap-4",
                        EqCard { class: "p-4",
                            div { class: "flex items-center gap-2 mb-3",
                                svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" }
                                }
                                span { class: "text-sm font-semibold text-[var(--ds-text)]", "应用管理" }
                            }
                            div { class: "grid grid-cols-2 gap-1.5",
                                DeviceToolCard { label: "启动应用", icon: "start", onclick: move |_| exec_adb("启动应用") }
                                DeviceToolCard { label: "停止应用", icon: "stop", onclick: move |_| exec_adb("停止应用") }
                            }
                        }

                        EqCard { class: "p-4",
                            div { class: "flex items-center gap-2 mb-3",
                                svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                                }
                                span { class: "text-sm font-semibold text-[var(--ds-text)]", "系统操作" }
                            }
                            div { class: "grid grid-cols-2 gap-1.5",
                                DeviceToolCard { label: "重启设备", icon: "reboot", onclick: move |_| exec_adb("重启设备") }
                                DeviceToolCard { label: "关闭设备", icon: "shutdown", onclick: move |_| exec_adb("关闭设备") }
                            }
                        }
                    }
                }
            }

            div { class: "md:hidden space-y-3",
                EqCard { class: "p-4",
                    div { class: "flex items-center gap-2 mb-3",
                        svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" }
                        }
                        span { class: "text-sm font-semibold text-[var(--ds-text)]", "ADB 命令" }
                    }
                    div { class: "flex gap-2 mb-3",
                        input {
                            class: "flex-1 min-h-[36px] px-2.5 border border-[var(--ds-border)] rounded bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] font-mono outline-none focus:border-[var(--ds-blue)]",
                            placeholder: "输入命令...",
                        }
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            "执行"
                        }
                    }
                    div { class: "grid grid-cols-5 gap-1.5",
                        AdbCommandCard { label: "唤醒", onclick: move |_| exec_adb("唤醒") }
                        AdbCommandCard { label: "息屏", onclick: move |_| exec_adb("息屏") }
                        AdbCommandCard { label: "解锁", onclick: move |_| exec_adb("解锁") }
                        AdbCommandCard { label: "Home", onclick: move |_| exec_adb("Home") }
                        AdbCommandCard { label: "返回", onclick: move |_| exec_adb("返回") }
                    }
                }

                EqCard { class: "p-4",
                    div { class: "flex items-center justify-between mb-3",
                        div { class: "flex items-center gap-2",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" }
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "设备信息" }
                        }
                        button {
                            class: "p-1 hover:bg-[var(--ds-surface)] rounded transition-colors",
                            onclick: move |_| load_device_info(),
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                            }
                        }
                    }
                    if *is_refreshing.read() {
                        div { class: "flex items-center justify-center py-3",
                            div { class: "w-4 h-4 border-2 border-[var(--ds-border)] border-t-[var(--ds-blue)] rounded-full animate-spin" }
                        }
                    } else if let Some(info) = device_info.read().as_ref() {
                        div { class: "grid grid-cols-3 gap-2",
                            div { class: "bg-[var(--ds-bg)] rounded p-2 text-center",
                                span { class: "block text-[10px] text-[var(--ds-text-tertiary)] mb-1", "型号" }
                                span { class: "text-xs text-[var(--ds-text)] font-mono truncate block", info.model.clone() }
                            }
                            div { class: "bg-[var(--ds-bg)] rounded p-2 text-center",
                                span { class: "block text-[10px] text-[var(--ds-text-tertiary)] mb-1", "电量" }
                                span { class: "text-xs text-[var(--ds-text)] font-mono", info.battery.clone() }
                            }
                            div { class: "bg-[var(--ds-bg)] rounded p-2 text-center",
                                span { class: "block text-[10px] text-[var(--ds-text-tertiary)] mb-1", "分辨率" }
                                span { class: "text-xs text-[var(--ds-text)] font-mono", info.screen_size.clone() }
                            }
                            div { class: "bg-[var(--ds-bg)] rounded p-2 text-center",
                                span { class: "block text-[10px] text-[var(--ds-text-tertiary)] mb-1", "WiFi" }
                                span { class: "text-xs text-[var(--ds-text)] font-mono truncate block", info.wifi.clone() }
                            }
                            div { class: "bg-[var(--ds-bg)] rounded p-2 text-center",
                                span { class: "block text-[10px] text-[var(--ds-text-tertiary)] mb-1", "IP" }
                                span { class: "text-xs text-[var(--ds-text)] font-mono", info.ip.clone() }
                            }
                            div { class: "bg-[var(--ds-bg)] rounded p-2 text-center",
                                span { class: "block text-[10px] text-[var(--ds-text-tertiary)] mb-1", "Android" }
                                span { class: "text-xs text-[var(--ds-text)] font-mono", info.android_version.clone() }
                            }
                        }
                    } else {
                        div { class: "text-center py-2",
                            span { class: "text-xs text-[var(--ds-text-tertiary)]", "点击刷新获取设备信息" }
                        }
                    }
                }

                div { class: "flex items-center justify-center min-w-0",
                    div { class: "relative border border-[var(--ds-border)] rounded-lg overflow-hidden bg-[#0a0a0f] flex items-center justify-center w-full max-w-[300px] aspect-[9/16] max-h-[60vh] mx-auto shadow-lg",
                        if *is_connected.read() {
                            div { class: "flex flex-col items-center gap-3 p-10 text-center",
                                p { class: "text-sm text-[var(--ds-text-secondary)]", "投屏中..." }
                            }
                        } else {
                            div { class: "flex flex-col items-center gap-3 p-10 text-center",
                                div { class: "w-16 h-16 flex items-center justify-center bg-[var(--ds-surface)] border border-[var(--ds-border)] rounded-2xl text-2xl text-[var(--ds-text-tertiary)] opacity-60",
                                    svg { class: "w-7 h-7", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" }
                                    }
                                }
                                p { class: "text-sm font-semibold text-[var(--ds-text-secondary)]", "设备未连接" }
                                p { class: "text-xs text-[var(--ds-text-tertiary)]", "点击上方\"开始投屏\"连接设备屏幕" }
                            }
                        }
                    }
                }

                div { class: "grid grid-cols-2 gap-3",
                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M7 21a4 4 0 01-4-4V5a2 2 0 012-2h4a2 2 0 012 2v12a4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "应用管理" }
                        }
                        div { class: "grid grid-cols-2 gap-1.5",
                            DeviceToolCard { label: "启动", icon: "start" }
                            DeviceToolCard { label: "停止", icon: "stop" }
                        }
                    }

                    EqCard { class: "p-4",
                        div { class: "flex items-center gap-2 mb-3",
                            svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
                            }
                            span { class: "text-sm font-semibold text-[var(--ds-text)]", "系统操作" }
                        }
                        div { class: "grid grid-cols-2 gap-1.5",
                            DeviceToolCard { label: "重启", icon: "reboot" }
                            DeviceToolCard { label: "关闭", icon: "shutdown" }
                        }
                    }
                }

                EqCard { class: "p-4",
                    div { class: "flex items-center gap-2 mb-3",
                        svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 17v-2m3 2v-4m3 4v-6m2 10H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" }
                        }
                        span { class: "text-sm font-semibold text-[var(--ds-text)]", "命令输出" }
                    }
                    div { class: "min-h-[80px] max-h-[150px] overflow-y-auto p-2.5 bg-[var(--ds-bg)] border border-[var(--ds-border)] rounded text-xs font-mono text-[var(--ds-text-secondary)] whitespace-pre-wrap break-all",
                        "等待执行命令..."
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct DeviceInfoItemProps {
    label: &'static str,
    value: String,
}

#[component]
fn DeviceInfoItem(props: DeviceInfoItemProps) -> Element {
    rsx! {
        div { class: "flex justify-between items-center text-xs",
            span { class: "text-[var(--ds-text-tertiary)]", "{props.label}" }
            span { class: "text-[var(--ds-text)] font-mono", if props.value.is_empty() { "--" } else { props.value } }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct AdbCommandCardProps {
    label: &'static str,
    onclick: EventHandler<()>,
}

#[component]
fn AdbCommandCard(props: AdbCommandCardProps) -> Element {
    rsx! {
        button {
            class: "flex items-center justify-center gap-1.5 px-2 py-2 bg-[var(--ds-surface)] border border-[var(--ds-border)] rounded cursor-pointer text-[11px] text-[var(--ds-text-secondary)] transition-all hover:bg-[var(--ds-blue-light)] hover:border-[var(--ds-blue)] hover:text-[var(--ds-blue)] active:scale-95",
            onclick: move |_| props.onclick.call(()),
            "{props.label}"
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct DeviceToolCardProps {
    label: &'static str,
    icon: &'static str,
    onclick: EventHandler<()>,
}

#[component]
fn DeviceToolCard(props: DeviceToolCardProps) -> Element {
    rsx! {
        button {
            class: "flex flex-col items-center gap-1 px-2 py-2.5 bg-[var(--ds-surface)] border border-[var(--ds-border)] rounded cursor-pointer text-[10px] text-[var(--ds-text-secondary)] transition-all hover:bg-[var(--ds-blue-light)] hover:border-[var(--ds-blue)] hover:text-[var(--ds-blue)] active:scale-95",
            onclick: move |_| props.onclick.call(()),
            DeviceToolIcon { icon: props.icon },
            "{props.label}"
        }
    }
}

#[component]
fn DeviceToolIcon(icon: &'static str) -> Element {
    match icon {
        "screencap" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z" }
            }
        },
        "battery" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707M16 12a4 4 0 11-8 0 4 4 0 018 0z" }
            }
        },
        "device" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 18h.01M8 21h8a2 2 0 002-2V5a2 2 0 00-2-2H8a2 2 0 00-2 2v14a2 2 0 002 2z" }
            }
        },
        "resolution" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" }
            }
        },
        "wifi" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.141 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" }
            }
        },
        "apps" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" }
            }
        },
        "start" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" }
            }
        },
        "stop" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" }
            }
        },
        "reboot" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" }
            }
        },
        "shutdown" => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" }
            }
        },
        _ => rsx! {
            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" }
            }
        },
    }
}