use dioxus::prelude::*;
use eq_ui::prelude::*;
use crate::api::client::{get_device_info, get_app_status, DeviceInfo, AppStatus};

#[component]
pub fn DashboardPage() -> Element {
    let mut device_info = use_signal(|| Option::<DeviceInfo>::None);
    let mut app_status = use_signal(|| Option::<AppStatus>::None);
    let mut loading = use_signal(|| false);
    let mut refresh = use_signal(|| 0u32);

    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            
            let (device_res, app_res) = tokio::join!(
                get_device_info(),
                get_app_status()
            );
            
            match device_res {
                Ok(info) => device_info.set(Some(info)),
                Err(_) => device_info.set(None),
            }
            
            match app_res {
                Ok(status) => app_status.set(Some(status)),
                Err(_) => app_status.set(None),
            }
            
            loading.set(false);
        });
    });

    let battery = device_info.read().as_ref().map(|d| d.battery.clone()).unwrap_or("--%".to_string());
    let storage = device_info.read().as_ref().map(|d| d.storage.clone()).unwrap_or("--".to_string());
    let tasks_count = app_status.read().as_ref().map(|s| s.tasks_count.to_string()).unwrap_or("--".to_string());
    let screenshots_count = app_status.read().as_ref().map(|s| s.screenshots_count.to_string()).unwrap_or("--".to_string());

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
                    value: battery,
                    detail: "电量状态",
                }
                StatusCard {
                    label: "存储",
                    value: storage,
                    detail: "磁盘使用",
                }
                StatusCard {
                    label: "任务",
                    value: tasks_count,
                    detail: "定时任务数",
                }
                StatusCard {
                    label: "截图",
                    value: screenshots_count,
                    detail: "已保存截图",
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
