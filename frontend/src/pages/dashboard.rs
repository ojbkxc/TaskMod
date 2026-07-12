use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn DashboardPage() -> Element {
    let mut battery = use_signal(|| "--%".to_string());
    let mut battery_status = use_signal(|| String::new());
    let mut uptime = use_signal(|| "--".to_string());
    let mut disk = use_signal(|| "--".to_string());
    let mut tasks_count = use_signal(|| "--".to_string());
    let mut screenshots_count = use_signal(|| "--".to_string());
    let mut loading = use_signal(|| false);
    let mut refresh = use_signal(|| 0u32);

    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            match crate::api::client::get_status().await {
                Ok(status) => {
                    if let Some(bat) = status.battery {
                        battery.set(bat.capacity.clone());
                        battery_status.set(bat.status.clone());
                    }
                    if let Some(u) = status.uptime {
                        uptime.set(u);
                    }
                    if let Some(d) = status.disk {
                        disk.set(d);
                    }
                    if let Some(t) = status.tasks_count {
                        tasks_count.set(t.to_string());
                    }
                    if let Some(s) = status.screenshots_count {
                        screenshots_count.set(s.to_string());
                    }
                }
                Err(_) => {}
            }
            loading.set(false);
        });
    });

    rsx! {
        div { class: "p-4 space-y-4",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "仪表盘" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "系统状态概览与快捷操作" }
                }
            }

            div { class: "grid grid-cols-2 md:grid-cols-4 gap-2",
                StatusCard {
                    label: "电池",
                    value: battery(),
                    detail: battery_status(),
                }
                StatusCard {
                    label: "任务",
                    value: tasks_count(),
                    detail: "定时任务数",
                }
                StatusCard {
                    label: "截图",
                    value: screenshots_count(),
                    detail: "已保存截图",
                }
                StatusCard {
                    label: "运行",
                    value: uptime(),
                    detail: "运行时间",
                }
            }

            div { class: "flex gap-2 flex-wrap",
                EqButton {
                    variant: EqButtonVariant::Secondary,
                    onclick: move |_| refresh += 1,
                    if *loading.read() { "刷新中..." } else { "刷新状态" }
                }
                EqButton {
                    variant: EqButtonVariant::Secondary,
                    onclick: move |_| {
                        spawn(async move {
                            let _ = crate::api::client::clear_logs().await;
                        });
                    },
                    "清除日志"
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct StatusCardProps {
    label: &'static str,
    value: String,
    detail: String,
}

#[component]
fn StatusCard(props: StatusCardProps) -> Element {
    rsx! {
        EqCard {
            class: "p-3 hover:border-[var(--ds-border-hover)] transition-all",
            div { class: "text-[10px] font-bold uppercase tracking-wider text-[var(--ds-text-tertiary)]",
                "{props.label}"
            }
            div { class: "mt-2 text-2xl font-bold text-[var(--ds-text)]",
                "{props.value}"
            }
            div { class: "mt-1.5 text-[11px] text-[var(--ds-text-tertiary)]",
                "{props.detail}"
            }
        }
    }
}
