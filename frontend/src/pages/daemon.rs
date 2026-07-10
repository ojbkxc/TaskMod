//! 隧道管理页面
//!
//! 支持多隧道、多服务的增删改查和独立控制
//! 包含 Token 管理、服务绑定、状态监控

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::api::client::ApiClient;

/// 隧道信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TunnelInfo {
    pub name: String,
    pub token: String,
    pub enabled: bool,
    pub services: Vec<ServiceInfo>,
}

/// 服务信息
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub url: String,
    pub enabled: bool,
}

/// 进程状态
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessStatus {
    pub tunnel_name: String,
    pub pid: u32,
    pub uptime_secs: u64,
    pub is_alive: bool,
}

/// 隧道管理页面组件
pub fn DaemonPage(cx: Scope) -> Element {
    let tunnels = use_state::<Vec<TunnelInfo>>(cx, || Vec::new());
    let processes = use_state::<Vec<ProcessStatus>>(cx, || Vec::new());
    let loading = use_state(cx, || false);
    let error = use_state::<Option<String>>(cx, || None);
    let show_add_tunnel = use_state(cx, || false);
    let selected_tunnel = use_state::<Option<String>>(cx, || None);

    // 加载数据
    let load_data = move |_| {
        loading.set(true);
        error.set(None);

        cx.spawn({
            let tunnels = tunnels.clone();
            let processes = processes.clone();
            let loading = loading.clone();
            let error = error.clone();

            async move {
                // 加载隧道列表
                match ApiClient::get("/api/tunnels").await {
                    Ok(resp) => {
                        if let Ok(data) = resp.json::<serde_json::Value>().await {
                            if let Some(arr) = data["data"].as_array() {
                                let list: Vec<TunnelInfo> = arr
                                    .iter()
                                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                                    .collect();
                                tunnels.set(list);
                            }
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("加载隧道失败: {}", e)));
                    }
                }

                // 加载进程状态
                match ApiClient::get("/api/processes").await {
                    Ok(resp) => {
                        if let Ok(data) = resp.json::<serde_json::Value>().await {
                            if let Some(arr) = data["data"].as_array() {
                                let list: Vec<ProcessStatus> = arr
                                    .iter()
                                    .filter_map(|v| serde_json::from_value(v.clone()).ok())
                                    .collect();
                                processes.set(list);
                            }
                        }
                    }
                    Err(_) => {}
                }

                loading.set(false);
            }
        });
    };

    // 初始加载
    use_effect(cx, (), |_| {
        load_data(());
        async {}
    });

    // 格式化运行时长
    let format_uptime = |secs: u64| -> String {
        match secs {
            s if s < 60 => format!("{}秒", s),
            s if s < 3600 => format!("{}分{}秒", s / 60, s % 60),
            s if s < 86400 => format!("{}时{}分", s / 3600, (s % 3600) / 60),
            s => format!("{}天{}时", s / 86400, (s % 86400) / 3600),
        }
    };

    // 获取进程状态
    let get_process = |tunnel_name: &str| -> Option<&ProcessStatus> {
        processes.iter().find(|p| p.tunnel_name == tunnel_name)
    };

    cx.render(rsx! {
        div {
            class: "p-6 max-w-6xl mx-auto",
            div {
                class: "flex justify-between items-center mb-6",
                h1 {
                    class: "text-2xl font-bold",
                    "隧道管理"
                }
                div {
                    class: "flex space-x-2",
                    button {
                        class: "px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600",
                        onclick: move |_| show_add_tunnel.set(true),
                        "添加隧道"
                    }
                    button {
                        class: "px-4 py-2 bg-gray-100 rounded hover:bg-gray-200",
                        onclick: load_data,
                        "刷新"
                    }
                }
            }

            // 错误提示
            if let Some(err) = error.get() {
                rsx! {
                    div {
                        class: "bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded mb-4",
                        "{err}"
                    }
                }
            }

            // 添加隧道对话框
            if *show_add_tunnel.get() {
                rsx! {
                    AddTunnelDialog {
                        on_close: move |_| show_add_tunnel.set(false),
                        on_success: move |_| {
                            show_add_tunnel.set(false);
                            load_data(());
                        }
                    }
                }
            }

            // 隧道列表
            if tunnels.is_empty() && !*loading.get() {
                rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-md p-8 text-center text-gray-500",
                        p { "暂无隧道配置" }
                        p { class: "text-sm mt-2", "点击「添加隧道」开始配置" }
                    }
                }
            } else {
                rsx! {
                    div {
                        class: "space-y-4",
                        for tunnel in tunnels.get() {
                            TunnelCard {
                                key: "{tunnel.name}",
                                tunnel: tunnel.clone(),
                                process: get_process(&tunnel.name).cloned(),
                                format_uptime: format_uptime,
                                on_refresh: move |_| load_data(()),
                            }
                        }
                    }
                }
            }
        }
    })
}

/// 隧道卡片组件
#[component]
fn TunnelCard<'a>(
    cx: Scope,
    tunnel: TunnelInfo,
    process: Option<ProcessStatus>,
    format_uptime: fn(u64) -> String,
    on_refresh: EventHandler<'a, ()>,
) -> Element {
    let expanded = use_state(cx, || false);
    let loading = use_state(cx, || false);

    let tunnel_name = tunnel.name.clone();
    let tunnel_name2 = tunnel.name.clone();
    let tunnel_name3 = tunnel.name.clone();
    let tunnel_name4 = tunnel.name.clone();
    let tunnel_name5 = tunnel.name.clone();

    // 启用/禁用隧道
    let toggle_enabled = move |_| {
        let name = tunnel_name.clone();
        let enabled = !tunnel.enabled;
        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_refresh = on_refresh.clone();

            async move {
                let url = if enabled {
                    format!("/api/tunnels/{}/enable", name)
                } else {
                    format!("/api/tunnels/{}/disable", name)
                };
                let _ = ApiClient::post(&url, "").await;
                loading.set(false);
                on_refresh.call(());
            }
        });
    };

    // 启动隧道
    let start_tunnel = move |_| {
        let name = tunnel_name2.clone();
        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_refresh = on_refresh.clone();

            async move {
                let _ = ApiClient::post(&format!("/api/tunnels/{}/start", name), "").await;
                loading.set(false);
                on_refresh.call(());
            }
        });
    };

    // 停止隧道
    let stop_tunnel = move |_| {
        let name = tunnel_name3.clone();
        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_refresh = on_refresh.clone();

            async move {
                let _ = ApiClient::post(&format!("/api/tunnels/{}/stop", name), "").await;
                loading.set(false);
                on_refresh.call(());
            }
        });
    };

    // 重启隧道
    let restart_tunnel = move |_| {
        let name = tunnel_name4.clone();
        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_refresh = on_refresh.clone();

            async move {
                let _ = ApiClient::post(&format!("/api/tunnels/{}/restart", name), "").await;
                loading.set(false);
                on_refresh.call(());
            }
        });
    };

    // 删除隧道
    let delete_tunnel = move |_| {
        let name = tunnel_name5.clone();
        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_refresh = on_refresh.clone();

            async move {
                let _ = ApiClient::delete(&format!("/api/tunnels/{}", name)).await;
                loading.set(false);
                on_refresh.call(());
            }
        });
    };

    let is_running = process.as_ref().map(|p| p.is_alive).unwrap_or(false);
    let pid = process.as_ref().map(|p| p.pid).unwrap_or(0);
    let uptime = process.as_ref().map(|p| p.uptime_secs).unwrap_or(0);

    cx.render(rsx! {
        div {
            class: "bg-white rounded-lg shadow-md overflow-hidden",
            // 头部
            div {
                class: "p-4 flex items-center justify-between",
                div {
                    class: "flex items-center space-x-3",
                    // 状态指示器
                    div {
                        class: format_args!("w-3 h-3 rounded-full {}",
                            if is_running { "bg-green-500" } else if tunnel.enabled { "bg-yellow-500" } else { "bg-gray-400" }
                        )
                    }
                    div {
                        h3 {
                            class: "font-semibold text-lg",
                            "{tunnel.name}"
                        }
                        p {
                            class: "text-sm text-gray-500",
                            if is_running {
                                "运行中 - PID: {pid} - {format_uptime(uptime)}"
                            } else if tunnel.enabled {
                                "已启用 - 未运行"
                            } else {
                                "已禁用"
                            }
                        }
                    }
                }
                div {
                    class: "flex items-center space-x-2",
                    // 启用/禁用开关
                    button {
                        class: format_args!("px-3 py-1 rounded text-sm {}",
                            if tunnel.enabled { "bg-green-100 text-green-700" } else { "bg-gray-100 text-gray-700" }
                        ),
                        onclick: toggle_enabled,
                        disabled: "{loading}",
                        if tunnel.enabled { "启用中" } else { "已禁用" }
                    }
                    // 展开/收起
                    button {
                        class: "px-2 py-1 text-gray-500 hover:text-gray-700",
                        onclick: move |_| expanded.set(!expanded),
                        if *expanded.get() { "收起" } else { "展开" }
                    }
                }
            }

            // 控制按钮
            div {
                class: "px-4 pb-3 flex space-x-2",
                if is_running {
                    rsx! {
                        button {
                            class: "px-3 py-1 bg-red-500 text-white rounded text-sm hover:bg-red-600 disabled:opacity-50",
                            onclick: stop_tunnel,
                            disabled: "{loading}",
                            "停止"
                        }
                        button {
                            class: "px-3 py-1 bg-blue-500 text-white rounded text-sm hover:bg-blue-600 disabled:opacity-50",
                            onclick: restart_tunnel,
                            disabled: "{loading}",
                            "重启"
                        }
                    }
                } else {
                    rsx! {
                        button {
                            class: "px-3 py-1 bg-green-500 text-white rounded text-sm hover:bg-green-600 disabled:opacity-50",
                            onclick: start_tunnel,
                            disabled: "{loading}",
                            "启动"
                        }
                    }
                }
                button {
                    class: "px-3 py-1 bg-gray-200 text-gray-700 rounded text-sm hover:bg-gray-300 disabled:opacity-50",
                    onclick: delete_tunnel,
                    disabled: "{loading}",
                    "删除"
                }
            }

            // 展开的服务列表
            if *expanded.get() {
                rsx! {
                    div {
                        class: "border-t border-gray-100 p-4",
                        h4 {
                            class: "font-medium mb-3",
                            "绑定的服务"
                        }
                        if tunnel.services.is_empty() {
                            rsx! {
                                p {
                                    class: "text-sm text-gray-500",
                                    "暂无服务配置"
                                }
                            }
                        } else {
                            rsx! {
                                div {
                                    class: "space-y-2",
                                    for service in &tunnel.services {
                                        ServiceItem {
                                            key: "{service.name}",
                                            tunnel_name: tunnel.name.clone(),
                                            service: service.clone(),
                                            on_refresh: move |_| on_refresh.call(()),
                                        }
                                    }
                                }
                            }
                        }
                        AddServiceForm {
                            tunnel_name: tunnel.name.clone(),
                            on_success: move |_| on_refresh.call(()),
                        }
                    }
                }
            }
        }
    })
}

/// 服务项组件
#[component]
fn ServiceItem<'a>(
    cx: Scope,
    tunnel_name: String,
    service: ServiceInfo,
    on_refresh: EventHandler<'a, ()>,
) -> Element {
    let loading = use_state(cx, || false);

    let tn = tunnel_name.clone();
    let sn = service.name.clone();

    // 启用/禁用服务
    let toggle_service = move |_| {
        let tn = tn.clone();
        let sn = sn.clone();
        let enabled = !service.enabled;
        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_refresh = on_refresh.clone();

            async move {
                let url = if enabled {
                    format!("/api/tunnels/{}/services/{}/enable", tn, sn)
                } else {
                    format!("/api/tunnels/{}/services/{}/disable", tn, sn)
                };
                let _ = ApiClient::post(&url, "").await;
                loading.set(false);
                on_refresh.call(());
            }
        });
    };

    let tn2 = tunnel_name.clone();
    let sn2 = service.name.clone();

    // 删除服务
    let delete_service = move |_| {
        let tn = tn2.clone();
        let sn = sn2.clone();
        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_refresh = on_refresh.clone();

            async move {
                let _ = ApiClient::delete(&format!("/api/tunnels/{}/services/{}", tn, sn)).await;
                loading.set(false);
                on_refresh.call(());
            }
        });
    };

    cx.render(rsx! {
        div {
            class: "flex items-center justify-between p-3 bg-gray-50 rounded",
            div {
                class: "flex items-center space-x-3",
                div {
                    class: format_args!("w-2 h-2 rounded-full {}", if service.enabled { "bg-green-500" } else { "bg-gray-400" })
                }
                div {
                    p {
                        class: "font-medium",
                        "{service.name}"
                    }
                    p {
                        class: "text-sm text-gray-500",
                        "{service.url}"
                    }
                }
            }
            div {
                class: "flex space-x-2",
                button {
                    class: format_args!("px-2 py-1 rounded text-xs {}",
                        if service.enabled { "bg-green-100 text-green-700" } else { "bg-gray-100 text-gray-700" }
                    ),
                    onclick: toggle_service,
                    disabled: "{loading}",
                    if service.enabled { "启用" } else { "禁用" }
                }
                button {
                    class: "px-2 py-1 bg-red-100 text-red-700 rounded text-xs hover:bg-red-200",
                    onclick: delete_service,
                    disabled: "{loading}",
                    "删除"
                }
            }
        }
    })
}

/// 添加服务表单
#[component]
fn AddServiceForm<'a>(
    cx: Scope,
    tunnel_name: String,
    on_success: EventHandler<'a, ()>,
) -> Element {
    let show_form = use_state(cx, || false);
    let name = use_state(cx, String::new);
    let url = use_state(cx, String::new);
    let loading = use_state(cx, || false);

    let tn = tunnel_name.clone();

    let submit = move |_| {
        let tn = tn.clone();
        let name = name.get().clone();
        let url = url.get().clone();

        if name.is_empty() || url.is_empty() {
            return;
        }

        loading.set(true);

        cx.spawn({
            let loading = loading.clone();
            let on_success = on_success.clone();
            let show_form = show_form.clone();

            async move {
                let body = serde_json::json!({
                    "name": name,
                    "url": url,
                    "enabled": true
                });
                let _ = ApiClient::post(
                    &format!("/api/tunnels/{}/services", tn),
                    &body.to_string(),
                ).await;
                loading.set(false);
                show_form.set(false);
                on_success.call(());
            }
        });
    };

    if *show_form.get() {
        cx.render(rsx! {
            div {
                class: "mt-3 p-3 bg-gray-50 rounded",
                div {
                    class: "flex space-x-2 mb-2",
                    input {
                        class: "flex-1 px-3 py-2 border rounded text-sm",
                        placeholder: "服务名称",
                        value: "{name}",
                        oninput: move |e| name.set(e.value.clone()),
                    }
                    input {
                        class: "flex-1 px-3 py-2 border rounded text-sm",
                        placeholder: "http://localhost:8080",
                        value: "{url}",
                        oninput: move |e| url.set(e.value.clone()),
                    }
                }
                div {
                    class: "flex justify-end space-x-2",
                    button {
                        class: "px-3 py-1 bg-gray-200 rounded text-sm",
                        onclick: move |_| show_form.set(false),
                        "取消"
                    }
                    button {
                        class: "px-3 py-1 bg-blue-500 text-white rounded text-sm",
                        onclick: submit,
                        disabled: "{loading}",
                        "添加"
                    }
                }
            }
        })
    } else {
        cx.render(rsx! {
            button {
                class: "mt-3 text-sm text-blue-500 hover:text-blue-700",
                onclick: move |_| show_form.set(true),
                "+ 添加服务"
            }
        })
    }
}

/// 添加隧道对话框
#[component]
fn AddTunnelDialog<'a>(
    cx: Scope,
    on_close: EventHandler<'a, ()>,
    on_success: EventHandler<'a, ()>,
) -> Element {
    let name = use_state(cx, String::new);
    let token = use_state(cx, String::new);
    let loading = use_state(cx, || false);
    let error = use_state::<Option<String>>(cx, || None);

    let submit = move |_| {
        let name = name.get().clone();
        let token = token.get().clone();

        if name.is_empty() || token.is_empty() {
            error.set(Some("名称和Token不能为空".to_string()));
            return;
        }

        loading.set(true);
        error.set(None);

        cx.spawn({
            let loading = loading.clone();
            let error = error.clone();
            let on_success = on_success.clone();

            async move {
                let body = serde_json::json!({
                    "name": name,
                    "token": token,
                    "enabled": true
                });
                match ApiClient::post("/api/tunnels", &body.to_string()).await {
                    Ok(resp) => {
                        if let Ok(data) = resp.json::<serde_json::Value>().await {
                            if data["success"].as_bool().unwrap_or(false) {
                                on_success.call(());
                            } else {
                                error.set(Some(
                                    data["error"].as_str().unwrap_or("添加失败").to_string(),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        error.set(Some(format!("请求失败: {}", e)));
                    }
                }
                loading.set(false);
            }
        });
    };

    cx.render(rsx! {
        div {
            class: "fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center z-50",
            div {
                class: "bg-white rounded-lg p-6 w-full max-w-md",
                h2 {
                    class: "text-xl font-bold mb-4",
                    "添加隧道"
                }
                if let Some(err) = error.get() {
                    rsx! {
                        div {
                            class: "bg-red-50 text-red-700 p-3 rounded mb-4 text-sm",
                            "{err}"
                        }
                    }
                }
                div {
                    class: "space-y-4",
                    div {
                        label {
                            class: "block text-sm font-medium mb-1",
                            "隧道名称"
                        }
                        input {
                            class: "w-full px-3 py-2 border rounded",
                            placeholder: "my-tunnel",
                            value: "{name}",
                            oninput: move |e| name.set(e.value.clone()),
                        }
                    }
                    div {
                        label {
                            class: "block text-sm font-medium mb-1",
                            "Tunnel Token"
                        }
                        textarea {
                            class: "w-full px-3 py-2 border rounded h-24 font-mono text-sm",
                            placeholder: "eyJhIjoixxxxxxxxx...",
                            value: "{token}",
                            oninput: move |e| token.set(e.value.clone()),
                        }
                    }
                }
                div {
                    class: "flex justify-end space-x-2 mt-6",
                    button {
                        class: "px-4 py-2 bg-gray-200 rounded hover:bg-gray-300",
                        onclick: move |_| on_close.call(()),
                        "取消"
                    }
                    button {
                        class: "px-4 py-2 bg-blue-500 text-white rounded hover:bg-blue-600 disabled:opacity-50",
                        onclick: submit,
                        disabled: "{loading}",
                        if *loading.get() { "添加中..." } else { "添加" }
                    }
                }
            }
        }
    })
}