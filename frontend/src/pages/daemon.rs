use dioxus::prelude::*;
use eq_ui::prelude::*;
use gloo_timers::future::sleep;
use std::time::Duration;
use serde_json::Value;
use crate::api::client::{
    list_tunnels, list_processes, add_tunnel, enable_tunnel, disable_tunnel,
    start_tunnel, stop_tunnel, restart_tunnel, delete_tunnel,
    list_services, add_service, enable_service, disable_service, delete_service,
    get_daemon_status, stop_daemon, restart_daemon,
    TunnelInfo, ServiceInfo, ProcessStatus,
};

fn format_uptime(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{}秒", s),
        s if s < 3600 => format!("{}分{}秒", s / 60, s % 60),
        s if s < 86400 => format!("{}时{}分", s / 3600, (s % 3600) / 60),
        s => format!("{}天{}时", s / 86400, (s % 86400) / 3600),
    }
}

#[derive(Debug, Clone, PartialEq)]
struct DaemonState {
    tunnels: Vec<TunnelInfo>,
    processes: Vec<ProcessStatus>,
    daemon_status: Value,
    loading: bool,
    error: Option<String>,
    show_add_tunnel: bool,
    auto_refresh: bool,
    last_refresh: u64,
}

impl Default for DaemonState {
    fn default() -> Self {
        Self {
            tunnels: Vec::new(),
            processes: Vec::new(),
            daemon_status: serde_json::json!({}),
            loading: false,
            error: None,
            show_add_tunnel: false,
            auto_refresh: true,
            last_refresh: 0,
        }
    }
}

#[component]
pub fn DaemonPage() -> Element {
    let state = use_signal(DaemonState::default);
    let refresh_trigger = use_signal(|| 0u32);

    let load_data = move || {
        let state = state.clone();
        async move {
            state.write().loading = true;
            state.write().error = None;

            let (tunnels_res, processes_res, status_res) = join(
                list_tunnels(),
                list_processes(),
                get_daemon_status()
            ).await;

            let mut s = state.write();
            match tunnels_res {
                Ok(list) => s.tunnels = list,
                Err(e) => s.error = Some(format!("加载隧道失败: {}", e)),
            }
            match processes_res {
                Ok(list) => s.processes = list,
                Err(e) => if s.error.is_none() { s.error = Some(format!("加载进程失败: {}", e)) },
            }
            match status_res {
                Ok(status) => s.daemon_status = status,
                Err(e) => if s.error.is_none() { s.error = Some(format!("加载守护进程状态失败: {}", e)) },
            }
            s.loading = false;
            s.last_refresh = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
        }
    };

    use_effect(move || {
        let _ = *refresh_trigger.read();
        spawn(async move {
            load_data().await;
        });
    });

    use_effect(move || {
        let state = state.clone();
        async move {
            loop {
                sleep(Duration::from_secs(5)).await;
                if state.read().auto_refresh && !state.read().loading {
                    spawn(async move {
                        load_data().await;
                    });
                }
            }
        }
    });

    let daemon_is_running = state.read().daemon_status.get("running")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let daemon_uptime = state.read().daemon_status.get("uptime")
        .and_then(|v| v.as_str())
        .unwrap_or("未知");

    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)] px-4",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "Cloudflare Tunnels" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-1", "隧道与服务配置、状态监控" }
                }
                div { class: "flex items-center gap-2",
                    div { class: "flex items-center gap-2 px-3 py-1.5 rounded-md",
                        class: if daemon_is_running {
                            "bg-[color-mix(in_srgb,var(--ds-success)_15%,transparent)]"
                        } else {
                            "bg-[color-mix(in_srgb,var(--ds-danger)_15%,transparent)]"
                        },
                        div {
                            class: "w-2 h-2 rounded-full animate-pulse",
                            background_color: if daemon_is_running {
                                "var(--ds-success)"
                            } else {
                                "var(--ds-danger)"
                            }
                        }
                        span { class: "text-xs font-medium",
                            if daemon_is_running {
                                format!("守护进程运行中 - {}", daemon_uptime)
                            } else {
                                "守护进程未运行".to_string()
                            }
                        }
                    }
                    EqButton {
                        variant: EqButtonVariant::Ghost,
                        size: EqButtonSize::Sm,
                        onclick: move |_| refresh_trigger.set(*refresh_trigger.read() + 1),
                        "刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        size: EqButtonSize::Sm,
                        onclick: move |_| state.write().show_add_tunnel = true,
                        "添加隧道"
                    }
                }
            }

            div { class: "flex-1 overflow-y-auto p-4 space-y-4",
                DaemonControl {
                    is_running: daemon_is_running,
                    on_stop: move |_| {
                        let state = state.clone();
                        spawn(async move {
                            let _ = stop_daemon().await;
                            load_data().await;
                        });
                    },
                    on_restart: move |_| {
                        let state = state.clone();
                        spawn(async move {
                            let _ = restart_daemon().await;
                            load_data().await;
                        });
                    },
                }

                if let Some(err) = state.read().error.clone() {
                    div { class: "p-3 border border-[var(--ds-danger)] bg-[color-mix(in_srgb,var(--ds-danger)_10%,transparent)] text-[var(--ds-danger)] rounded-md text-sm",
                        "{err}"
                    }
                }

                if *state.read().loading.read() {
                    div { class: "space-y-3",
                        for _ in 0..3 {
                            div { class: "h-44 bg-[var(--ds-surface)] rounded-lg animate-pulse" }
                        }
                    }
                } else if state.read().tunnels.is_empty() {
                    div { class: "flex flex-col items-center justify-center min-h-[320px] gap-2.5 p-6 text-center",
                        div { class: "flex items-center justify-center w-16 h-16 border-2 border-[var(--ds-border)] rounded-xl text-[var(--ds-text-tertiary)]",
                            svg { class: "w-8 h-8", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" }
                            }
                        }
                        p { class: "text-base font-semibold text-[var(--ds-text)]", "暂无隧道配置" }
                        p { class: "text-xs text-[var(--ds-text-tertiary)] max-w-xs", "点击\"添加隧道\"开始配置 Cloudflare Tunnel" }
                        div { class: "mt-4 text-xs text-[var(--ds-text-secondary)] bg-[var(--ds-surface)] px-3 py-2 rounded-md",
                            "提示: 需要先在 Cloudflare 控制台创建隧道并获取 Token"
                        }
                    }
                } else {
                    div { class: "space-y-3",
                        for tunnel in state.read().tunnels.clone() {
                            let process = state.read().processes
                                .iter()
                                .find(|p| p.tunnel_name == tunnel.name)
                                .cloned();
                            TunnelCard {
                                key: "{tunnel.name}",
                                tunnel: tunnel.clone(),
                                process: process,
                                on_refresh: move |_| refresh_trigger.set(*refresh_trigger.read() + 1),
                            }
                        }
                    }
                }
            }

            if *state.read().show_add_tunnel.read() {
                AddTunnelDialog {
                    on_close: move |_| state.write().show_add_tunnel = false,
                    on_success: move |_| {
                        state.write().show_add_tunnel = false;
                        refresh_trigger.set(*refresh_trigger.read() + 1);
                    }
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct DaemonControlProps {
    is_running: bool,
    on_stop: EventHandler<()>,
    on_restart: EventHandler<()>,
}

#[component]
fn DaemonControl(props: DaemonControlProps) -> Element {
    rsx! {
        EqCard {
            div { class: "flex items-center justify-between p-4",
                div {
                    h3 { class: "font-semibold text-[var(--ds-text)]", "守护进程控制" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)] mt-0.5",
                        "管理 taskmod-daemon 服务状态"
                    }
                }
                div { class: "flex gap-2",
                    if props.is_running {
                        rsx! {
                            EqButton {
                                variant: EqButtonVariant::Secondary,
                                size: EqButtonSize::Sm,
                                onclick: props.on_restart,
                                "重启"
                            }
                            EqButton {
                                variant: EqButtonVariant::Danger,
                                size: EqButtonSize::Sm,
                                onclick: props.on_stop,
                                "停止"
                            }
                        }
                    } else {
                        rsx! {
                            EqButton {
                                variant: EqButtonVariant::Primary,
                                size: EqButtonSize::Sm,
                                disabled: true,
                                "启动"
                            }
                        }
                    }
                }
            }
        }
    }
}

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

    let toggle_enabled = move |_| {
        let name = tunnel_name.clone();
        let enabled = !props.tunnel.enabled;
        let on_refresh = props.on_refresh.clone();
        loading.set(true);
        spawn(async move {
            if enabled {
                let _ = enable_tunnel(&name).await;
            } else {
                let _ = disable_tunnel(&name).await;
            }
            loading.set(false);
            on_refresh.call(());
        });
    };

    let start_tunnel_fn = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);
        spawn(async move {
            let _ = start_tunnel(&name).await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    let stop_tunnel_fn = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);
        spawn(async move {
            let _ = stop_tunnel(&name).await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    let restart_tunnel_fn = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);
        spawn(async move {
            let _ = restart_tunnel(&name).await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    let delete_tunnel_fn = move |_| {
        let name = tunnel_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);
        spawn(async move {
            let _ = delete_tunnel(&name).await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    let is_running = props.process.as_ref().map(|p| p.is_alive).unwrap_or(false);
    let pid = props.process.as_ref().map(|p| p.pid).unwrap_or(0);
    let uptime = props.process.as_ref().map(|p| p.uptime_secs).unwrap_or(0);

    rsx! {
        EqCard { class: "overflow-hidden transition-all",
            div { class: "p-4 flex items-center justify-between",
                div { class: "flex items-center space-x-3",
                    div { class: "relative",
                        div {
                            class: "w-3 h-3 rounded-full",
                            background_color: if is_running {
                                "var(--ds-success)"
                            } else if props.tunnel.enabled {
                                "var(--ds-warning)"
                            } else {
                                "var(--ds-text-tertiary)"
                            }
                        }
                        if is_running {
                            div {
                                class: "absolute inset-0 w-3 h-3 rounded-full bg-[var(--ds-success)] animate-ping opacity-75"
                            }
                        }
                    }
                    div {
                        h3 { class: "font-semibold text-[var(--ds-text)]", "{props.tunnel.name}" }
                        p { class: "text-xs text-[var(--ds-text-tertiary)]",
                            if is_running {
                                "运行中 · PID: {pid} · {format_uptime(uptime)}"
                            } else if props.tunnel.enabled {
                                "已启用 · 未运行"
                            } else {
                                "已禁用"
                            }
                        }
                    }
                }
                div { class: "flex items-center gap-2",
                    button {
                        class: format!("px-2.5 py-1 rounded text-xs font-medium transition-colors {}",
                            if props.tunnel.enabled {
                                "bg-[color-mix(in_srgb,var(--ds-success)_15%,transparent)] text-[var(--ds-success)] hover:bg-[color-mix(in_srgb,var(--ds-success)_25%,transparent)]"
                            } else {
                                "bg-[var(--ds-surface)] text-[var(--ds-text-tertiary)] hover:bg-[color-mix(in_srgb,var(--ds-text-tertiary)_10%,transparent)]"
                            }
                        ),
                        onclick: toggle_enabled,
                        disabled: *loading.read(),
                        if props.tunnel.enabled { "启用中" } else { "已禁用" }
                    }
                    button {
                        class: "p-1.5 text-[var(--ds-text-tertiary)] hover:text-[var(--ds-text)] hover:bg-[var(--ds-surface)] rounded transition-colors",
                        onclick: move |_| expanded.set(!*expanded.read()),
                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: if *expanded.read() { "M5 15l7-7 7 7" } else { "M19 9l-7 7-7-7" } }
                        }
                    }
                }
            }

            div { class: "px-4 pb-3 flex gap-2",
                if is_running {
                    rsx! {
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            size: EqButtonSize::Sm,
                            onclick: stop_tunnel_fn,
                            disabled: *loading.read(),
                            "停止"
                        }
                        EqButton {
                            variant: EqButtonVariant::Secondary,
                            size: EqButtonSize::Sm,
                            onclick: restart_tunnel_fn,
                            disabled: *loading.read(),
                            "重启"
                        }
                    }
                } else if props.tunnel.enabled {
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        size: EqButtonSize::Sm,
                        onclick: start_tunnel_fn,
                        disabled: *loading.read(),
                        "启动"
                    }
                }
                EqButton {
                    variant: EqButtonVariant::Ghost,
                    size: EqButtonSize::Sm,
                    onclick: delete_tunnel_fn,
                    disabled: *loading.read(),
                    "删除"
                }
            }

            if *expanded.read() {
                div { class: "border-t border-[var(--ds-border)] p-4",
                    div { class: "grid grid-cols-3 gap-4 mb-4",
                        div { class: "bg-[var(--ds-surface)] rounded-lg p-3",
                            div { class: "text-[10px] text-[var(--ds-text-tertiary)] uppercase tracking-wider mb-1", "状态" }
                            div { class: "text-sm font-semibold",
                                if is_running {
                                    span { class: "text-[var(--ds-success)]", "运行中" }
                                } else {
                                    span { class: "text-[var(--ds-text-tertiary)]", "已停止" }
                                }
                            }
                        }
                        div { class: "bg-[var(--ds-surface)] rounded-lg p-3",
                            div { class: "text-[10px] text-[var(--ds-text-tertiary)] uppercase tracking-wider mb-1", "进程ID" }
                            div { class: "text-sm font-mono", if pid > 0 { "{pid}" } else { "-" } }
                        }
                        div { class: "bg-[var(--ds-surface)] rounded-lg p-3",
                            div { class: "text-[10px] text-[var(--ds-text-tertiary)] uppercase tracking-wider mb-1", "运行时间" }
                            div { class: "text-sm", "{format_uptime(uptime)}" }
                        }
                    }

                    h4 { class: "font-medium mb-3 text-[var(--ds-text)] flex items-center gap-2",
                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.375 16.5l-1.5 1.5L4.5 12l3.375-3 1.5 1.5L12 12l-1.5 1.5" }
                        }
                        "绑定的服务"
                    }
                    if props.tunnel.services.is_empty() {
                        div { class: "text-sm text-[var(--ds-text-tertiary)] bg-[var(--ds-surface)] p-3 rounded-md text-center",
                            "暂无服务配置"
                        }
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

    let toggle_service = move |_| {
        let tn = tunnel_name.clone();
        let sn = service_name.clone();
        let enabled = !props.service.enabled;
        let on_refresh = props.on_refresh.clone();
        loading.set(true);
        spawn(async move {
            if enabled {
                let _ = enable_service(&tn, &sn).await;
            } else {
                let _ = disable_service(&tn, &sn).await;
            }
            loading.set(false);
            on_refresh.call(());
        });
    };

    let delete_service_fn = move |_| {
        let tn = tunnel_name.clone();
        let sn = service_name.clone();
        let on_refresh = props.on_refresh.clone();
        loading.set(true);
        spawn(async move {
            let _ = delete_service(&tn, &sn).await;
            loading.set(false);
            on_refresh.call(());
        });
    };

    rsx! {
        div { class: "flex items-center justify-between p-3 bg-[var(--ds-surface)] rounded-lg hover:bg-[color-mix(in_srgb,var(--ds-surface)_80%,transparent)] transition-colors",
            div { class: "flex items-center space-x-3",
                div { class: "relative",
                    div {
                        class: "w-2 h-2 rounded-full",
                        background_color: if props.service.enabled {
                            "var(--ds-success)"
                        } else {
                            "var(--ds-text-tertiary)"
                        }
                    }
                    if props.service.enabled {
                        div {
                            class: "absolute inset-0 w-2 h-2 rounded-full bg-[var(--ds-success)] animate-ping opacity-50"
                        }
                    }
                }
                div {
                    p { class: "text-sm font-medium text-[var(--ds-text)]", "{props.service.name}" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)] font-mono", "{props.service.url}" }
                }
            }
            div { class: "flex gap-1.5",
                button {
                    class: format!("px-2 py-1 rounded text-xs font-medium transition-colors {}",
                        if props.service.enabled {
                            "bg-[color-mix(in_srgb,var(--ds-success)_15%,transparent)] text-[var(--ds-success)] hover:bg-[color-mix(in_srgb,var(--ds-success)_25%,transparent)]"
                        } else {
                            "bg-[var(--ds-bg)] text-[var(--ds-text-tertiary)] hover:bg-[color-mix(in_srgb,var(--ds-text-tertiary)_10%,transparent)]"
                        }
                    ),
                    onclick: toggle_service,
                    disabled: *loading.read(),
                    if props.service.enabled { "启用" } else { "禁用" }
                }
                button {
                    class: "px-2 py-1 bg-[color-mix(in_srgb,var(--ds-danger)_15%,transparent)] text-[var(--ds-danger)] rounded text-xs font-medium hover:bg-[color-mix(in_srgb,var(--ds-danger)_25%,transparent)] transition-colors",
                    onclick: delete_service_fn,
                    disabled: *loading.read(),
                    "删除"
                }
            }
        }
    }
}

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
            let _ = add_service(&tn, &n, &u, true).await;
            loading.set(false);
            show_form.set(false);
            name.set(String::new());
            url.set(String::new());
            on_success.call(());
        });
    };

    if *show_form.read() {
        rsx! {
            div { class: "mt-3 p-4 bg-[var(--ds-surface)] rounded-lg",
                div { class: "flex gap-2 mb-3",
                    input {
                        class: "flex-1 px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                        placeholder: "服务名称",
                        value: "{name}",
                        oninput: move |e| name.set(e.value.clone()),
                    }
                    input {
                        class: "flex-1 px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
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
                class: "mt-3 text-xs text-[var(--ds-blue)] hover:opacity-80 flex items-center gap-1",
                onclick: move |_| show_form.set(true),
                svg { class: "w-3.5 h-3.5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor",
                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 4v16m8-8H4" }
                }
                "+ 添加服务"
            }
        }
    }
}

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
            match add_tunnel(&n, &t, true).await {
                Ok(_) => on_success.call(()),
                Err(e) => error.set(Some(format!("添加失败: {}", e))),
            }
            loading.set(false);
        });
    };

    rsx! {
        div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
            div { class: "bg-[var(--ds-card)] rounded-xl p-6 w-full max-w-md border border-[var(--ds-border)] shadow-xl",
                h2 { class: "text-lg font-bold mb-4 text-[var(--ds-text)]", "添加 Cloudflare Tunnel" }
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
                            class: "w-full px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
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
                            class: "w-full px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] font-mono outline-none focus:border-[var(--ds-blue)] h-28",
                            placeholder: "eyJhIjoixxxxxxxxx...",
                            value: "{token}",
                            oninput: move |e| token.set(e.value.clone()),
                        }
                        p { class: "text-[10px] text-[var(--ds-text-tertiary)] mt-1",
                            "在 Cloudflare 控制台创建隧道后获取此 Token"
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