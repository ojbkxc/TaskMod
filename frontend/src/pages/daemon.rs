//! 隧道管理页面
//!
//! 支持多隧道、多服务的增删改查和独立控制
//! 包含 Token 管理、服务绑定、状态监控

use dioxus::prelude::*;
use eq_ui::prelude::*;
use serde::{Deserialize, Serialize};

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

/// 格式化运行时长
fn format_uptime(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{}秒", s),
        s if s < 3600 => format!("{}分{}秒", s / 60, s % 60),
        s if s < 86400 => format!("{}时{}分", s / 3600, (s % 3600) / 60),
        s => format!("{}天{}时", s / 86400, (s % 86400) / 3600),
    }
}

/// 隧道管理页面组件
#[component]
pub fn DaemonPage() -> Element {
    let mut tunnels = use_signal(|| Vec::<TunnelInfo>::new());
    let mut processes = use_signal(|| Vec::<ProcessStatus>::new());
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut show_add_tunnel = use_signal(|| false);
    let mut refresh = use_signal(|| 0u32);

    // 加载数据（由 refresh signal 触发）
    use_effect(move || {
        let _ = *refresh.read();

        spawn(async move {
            loading.set(true);
            error.set(None);

            // 加载隧道列表
            match reqwest::get("/api/tunnels").await {
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
            match reqwest::get("/api/processes").await {
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
        });
    });

    rsx! {
        div { class: "p-4 space-y-4",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "隧道管理" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "隧道与服务配置、状态监控" }
                }
                div { class: "flex gap-1.5",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        onclick: move |_| refresh.set(*refresh.read() + 1),
                        "刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        onclick: move |_| show_add_tunnel.set(true),
                        "添加隧道"
                    }
                }
            }

            // 错误提示
            if let Some(err) = error.read().clone() {
                div { class: "p-3 border border-[var(--ds-danger)] bg-[color-mix(in_srgb,var(--ds-danger)_10%,transparent)] text-[var(--ds-danger)] rounded-md text-sm",
                    "{err}"
                }
            }

            // 添加隧道对话框
            if *show_add_tunnel.read() {
                AddTunnelDialog {
                    on_close: move |_| show_add_tunnel.set(false),
                    on_success: move |_| {
                        show_add_tunnel.set(false);
                        refresh.set(*refresh.read() + 1);
                    }
                }
            }

            // 隧道列表
            if tunnels.read().is_empty() && !*loading.read() {
                div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2.5 p-6 text-center",
                    div { class: "flex items-center justify-center w-7 h-7 border border-[var(--ds-border)] rounded-md text-[var(--ds-text-tertiary)]",
                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" }
                        }
                    }
                    p { class: "text-sm font-semibold text-[var(--ds-text)]", "暂无隧道配置" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)]", "点击\"添加隧道\"开始配置" }
                }
            } else {
                div { class: "space-y-3",
                    for tunnel in tunnels.read().clone() {
                        let process = processes
                            .read()
                            .iter()
                            .find(|p| p.tunnel_name == tunnel.name)
                            .cloned();
                        TunnelCard {
                            key: "{tunnel.name}",
                            tunnel: tunnel.clone(),
                            process: process,
                            on_refresh: move |_| refresh.set(*refresh.read() + 1),
                        }
                    }
                }
            }
        }
    }
}

/// 隧道卡片组件
#[derive(Props, PartialEq, Clone)]
struct TunnelCardProps {
    tunnel: TunnelInfo,
    process: Option<ProcessStatus>,
    on_refresh: EventHandler<()>,
}

#[component]
fn TunnelCard(props: TunnelCardProps) -> Element {
    let mut expanded = use_signal(|| false);
    let mut loading = use_signal(|| false);

    let tunnel_name = props.tunnel.name.clone();

    // 启用/禁用隧道
    let toggle_enabled = move |_| {
        let name = tunnel_name.clone();
        let enabled = !props.tunnel.enabled;
        let on_refresh = props.on_refresh.clone();
        loading.set(true);

        spawn(async move {
            let url = if enabled {
                format!("/api/tunnels/{}/enable", name)
            } else {
                format!("/api/tunnels/{}/disable", name)
            };
            let _ = reqwest::Client::new().post(&url).body("").send().await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    // 启动隧道
    let start_tunnel = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);

        spawn(async move {
            let _ = reqwest::Client::new()
                .post(&format!("/api/tunnels/{}/start", name))
                .body("")
                .send()
                .await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    // 停止隧道
    let stop_tunnel = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);

        spawn(async move {
            let _ = reqwest::Client::new()
                .post(&format!("/api/tunnels/{}/stop", name))
                .body("")
                .send()
                .await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    // 重启隧道
    let restart_tunnel = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);

        spawn(async move {
            let _ = reqwest::Client::new()
                .post(&format!("/api/tunnels/{}/restart", name))
                .body("")
                .send()
                .await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    // 删除隧道
    let delete_tunnel = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);

        spawn(async move {
            let _ = reqwest::Client::new()
                .delete(&format!("/api/tunnels/{}", name))
                .send()
                .await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    let is_running = props.process.as_ref().map(|p| p.is_alive).unwrap_or(false);
    let pid = props.process.as_ref().map(|p| p.pid).unwrap_or(0);
    let uptime = props.process.as_ref().map(|p| p.uptime_secs).unwrap_or(0);

    rsx! {
        EqCard { class: "overflow-hidden",
            // 头部
            div { class: "p-4 flex items-center justify-between",
                div { class: "flex items-center space-x-3",
                    // 状态指示器
                    div {
                        class: "w-2.5 h-2.5 rounded-full",
                        background_color: if is_running {
                            "var(--ds-success)"
                        } else if props.tunnel.enabled {
                            "var(--ds-warning)"
                        } else {
                            "var(--ds-text-tertiary)"
                        }
                    }
                    div {
                        h3 { class: "font-semibold text-[var(--ds-text)]", "{props.tunnel.name}" }
                        p { class: "text-xs text-[var(--ds-text-tertiary)]",
                            if is_running {
                                "运行中 - PID: {pid} - {format_uptime(uptime)}"
                            } else if props.tunnel.enabled {
                                "已启用 - 未运行"
                            } else {
                                "已禁用"
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-2",
                    // 启用/禁用开关
                    button {
                        class: format!("px-2.5 py-1 rounded text-xs {}",
                            if props.tunnel.enabled {
                                "bg-[color-mix(in_srgb,var(--ds-success)_15%,transparent)] text-[var(--ds-success)]"
                            } else {
                                "bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)]"
                            }
                        ),
                        onclick: toggle_enabled,
                        disabled: *loading.read(),
                        if props.tunnel.enabled { "启用中" } else { "已禁用" }
                    }
                    // 展开/收起
                    button {
                        class: "px-2 py-1 text-[var(--ds-text-tertiary)] hover:text-[var(--ds-text)] text-xs",
                        onclick: move |_| expanded.set(!*expanded.read()),
                        if *expanded.read() { "收起" } else { "展开" }
                    }
                }
            }

            // 控制按钮
            div { class: "px-4 pb-3 flex gap-2",
                if is_running {
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        size: EqButtonSize::Sm,
                        onclick: stop_tunnel,
                        disabled: *loading.read(),
                        "停止"
                    }
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        size: EqButtonSize::Sm,
                        onclick: restart_tunnel,
                        disabled: *loading.read(),
                        "重启"
                    }
                } else {
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        size: EqButtonSize::Sm,
                        onclick: start_tunnel,
                        disabled: *loading.read(),
                        "启动"
                    }
                }
                EqButton {
                    variant: EqButtonVariant::Ghost,
                    size: EqButtonSize::Sm,
                    onclick: delete_tunnel,
                    disabled: *loading.read(),
                    "删除"
                }
            }

            // 展开的服务列表
            if *expanded.read() {
                div { class: "border-t border-[var(--ds-border)] p-4",
                    h4 { class: "font-medium mb-3 text-[var(--ds-text)]", "绑定的服务" }
                    if props.tunnel.services.is_empty() {
                        p { class: "text-sm text-[var(--ds-text-tertiary)]", "暂无服务配置" }
                    } else {
                        div { class: "space-y-2",
                            for service in props.tunnel.services.clone() {
                                ServiceItem {
                                    key: "{service.name}",
                                    tunnel_name: props.tunnel.name.clone(),
                                    service: service.clone(),
                                    on_refresh: props.on_refresh.clone(),
                                }
                            }
                        }
                    }
                    AddServiceForm {
                        tunnel_name: props.tunnel.name.clone(),
                        on_success: props.on_refresh.clone(),
                    }
                }
            }
        }
    }
}

/// 服务项组件
#[derive(Props, PartialEq, Clone)]
struct ServiceItemProps {
    tunnel_name: String,
    service: ServiceInfo,
    on_refresh: EventHandler<()>,
}

#[component]
fn ServiceItem(props: ServiceItemProps) -> Element {
    let mut loading = use_signal(|| false);

    let tunnel_name = props.tunnel_name.clone();
    let service_name = props.service.name.clone();

    // 启用/禁用服务
    let toggle_service = move |_| {
        let tn = tunnel_name.clone();
        let sn = service_name.clone();
        let enabled = !props.service.enabled;
        let on_refresh = props.on_refresh.clone();
        loading.set(true);

        spawn(async move {
            let url = if enabled {
                format!("/api/tunnels/{}/services/{}/enable", tn, sn)
            } else {
                format!("/api/tunnels/{}/services/{}/disable", tn, sn)
            };
            let _ = reqwest::Client::new().post(&url).body("").send().await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    let tn2 = tunnel_name.clone();
    let sn2 = service_name.clone();

    // 删除服务
    let delete_service = move |_| {
        let tn = tn2.clone();
        let sn = sn2.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);

        spawn(async move {
            let _ = reqwest::Client::new()
                .delete(&format!("/api/tunnels/{}/services/{}", tn, sn))
                .send()
                .await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    rsx! {
        div { class: "flex items-center justify-between p-2.5 bg-[var(--ds-surface)] rounded-md",
            div { class: "flex items-center space-x-3",
                div {
                    class: "w-2 h-2 rounded-full",
                    background_color: if props.service.enabled {
                        "var(--ds-success)"
                    } else {
                        "var(--ds-text-tertiary)"
                    }
                }
                div {
                    p { class: "text-sm font-medium text-[var(--ds-text)]", "{props.service.name}" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)] font-mono", "{props.service.url}" }
                }
            }
            div { class: "flex gap-1.5",
                button {
                    class: format!("px-2 py-1 rounded text-xs {}",
                        if props.service.enabled {
                            "bg-[color-mix(in_srgb,var(--ds-success)_15%,transparent)] text-[var(--ds-success)]"
                        } else {
                            "bg-[var(--ds-bg)] text-[var(--ds-text-tertiary)]"
                        }
                    ),
                    onclick: toggle_service,
                    disabled: *loading.read(),
                    if props.service.enabled { "启用" } else { "禁用" }
                }
                button {
                    class: "px-2 py-1 bg-[color-mix(in_srgb,var(--ds-danger)_15%,transparent)] text-[var(--ds-danger)] rounded text-xs hover:bg-[color-mix(in_srgb,var(--ds-danger)_25%,transparent)]",
                    onclick: delete_service,
                    disabled: *loading.read(),
                    "删除"
                }
            }
        }
    }
}

/// 添加服务表单
#[derive(Props, PartialEq, Clone)]
struct AddServiceFormProps {
    tunnel_name: String,
    on_success: EventHandler<()>,
}

#[component]
fn AddServiceForm(props: AddServiceFormProps) -> Element {
    let mut show_form = use_signal(|| false);
    let mut name = use_signal(String::new);
    let mut url = use_signal(String::new);
    let mut loading = use_signal(|| false);

    let tunnel_name = props.tunnel_name.clone();

    let submit = move |_| {
        let tn = tunnel_name.clone();
        let n = name.read().clone();
        let u = url.read().clone();

        if n.is_empty() || u.is_empty() {
            return;
        }

        let on_success = props.on_success.clone();
        loading.set(true);

        spawn(async move {
            let body = serde_json::json!({
                "name": n,
                "url": u,
                "enabled": true
            });
            let _ = reqwest::Client::new()
                .post(&format!("/api/tunnels/{}/services", tn))
                .json(&body)
                .send()
                .await;
            loading.set(false);
            show_form.set(false);
            name.set(String::new());
            url.set(String::new());
            on_success.call(());
        });
    };

    if *show_form.read() {
        rsx! {
            div { class: "mt-3 p-3 bg-[var(--ds-surface)] rounded-md",
                div { class: "flex gap-2 mb-2",
                    input {
                        class: "flex-1 px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                        placeholder: "服务名称",
                        value: "{name}",
                        oninput: move |e| name.set(e.value.clone()),
                    }
                    input {
                        class: "flex-1 px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                        placeholder: "http://localhost:8080",
                        value: "{url}",
                        oninput: move |e| url.set(e.value.clone()),
                    }
                }
                div { class: "flex justify-end gap-2",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        size: EqButtonSize::Sm,
                        onclick: move |_| show_form.set(false),
                        "取消"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        size: EqButtonSize::Sm,
                        onclick: submit,
                        disabled: *loading.read(),
                        "添加"
                    }
                }
            }
        }
    } else {
        rsx! {
            button {
                class: "mt-3 text-xs text-[var(--ds-blue)] hover:opacity-80",
                onclick: move |_| show_form.set(true),
                "+ 添加服务"
            }
        }
    }
}

/// 添加隧道对话框
#[derive(Props, PartialEq, Clone)]
struct AddTunnelDialogProps {
    on_close: EventHandler<()>,
    on_success: EventHandler<()>,
}

#[component]
fn AddTunnelDialog(props: AddTunnelDialogProps) -> Element {
    let mut name = use_signal(String::new);
    let mut token = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    let submit = move |_| {
        let n = name.read().clone();
        let t = token.read().clone();

        if n.is_empty() || t.is_empty() {
            error.set(Some("名称和Token不能为空".to_string()));
            return;
        }

        let on_success = props.on_success.clone();
        loading.set(true);
        error.set(None);

        spawn(async move {
            let body = serde_json::json!({
                "name": n,
                "token": t,
                "enabled": true
            });
            match reqwest::Client::new().post("/api/tunnels").json(&body).send().await {
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
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            div { class: "bg-[var(--ds-card)] rounded-lg p-6 w-full max-w-md border border-[var(--ds-border)] shadow-xl",
                h2 { class: "text-lg font-bold mb-4 text-[var(--ds-text)]", "添加隧道" }
                if let Some(err) = error.read().clone() {
                    div { class: "p-2.5 border border-[var(--ds-danger)] text-[var(--ds-danger)] rounded-md mb-4 text-sm bg-[color-mix(in_srgb,var(--ds-danger)_10%,transparent)]",
                        "{err}"
                    }
                }
                div { class: "space-y-4",
                    div {
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                            "隧道名称"
                        }
                        input {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                            placeholder: "my-tunnel",
                            value: "{name}",
                            oninput: move |e| name.set(e.value.clone()),
                        }
                    }
                    div {
                        label { class: "block text-[11px] font-bold text-[var(--ds-text)] uppercase tracking-wider mb-1",
                            "Tunnel Token"
                        }
                        textarea {
                            class: "w-full px-2.5 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] font-mono outline-none focus:border-[var(--ds-blue)] h-24",
                            placeholder: "eyJhIjoixxxxxxxxx...",
                            value: "{token}",
                            oninput: move |e| token.set(e.value.clone()),
                        }
                    }
                }
                div { class: "flex justify-end gap-2 mt-6",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        onclick: move |_| props.on_close.call(()),
                        "取消"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        onclick: submit,
                        disabled: *loading.read(),
                        if *loading.read() { "添加中..." } else { "添加" }
                    }
                }
            }
        }
    }
}
