use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn DashboardPage() -> Element {
    rsx! {
        div { class: "p-4 space-y-4",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "仪表盘" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "系统状态概览与快捷操作" }
                }
            }

            // 状态卡片网格
            div { class: "grid grid-cols-2 md:grid-cols-4 gap-2",
                StatusCard {
                    label: "电池",
                    value: "--%",
                    detail: "电量状态",
                }
                StatusCard {
                    label: "CPU",
                    value: "--%",
                    detail: "使用率",
                }
                StatusCard {
                    label: "内存",
                    value: "--",
                    detail: "MB 使用中",
                }
                StatusCard {
                    label: "运行",
                    value: "--",
                    detail: "运行时间",
                }
            }

            // 快捷操作
            div { class: "flex gap-2 flex-wrap",
                EqButton {
                    variant: EqButtonVariant::Secondary,
                    "刷新状态"
                }
                EqButton {
                    variant: EqButtonVariant::Secondary,
                    "清除日志"
                }
            }
        }
    }
}

#[derive(Props, PartialEq, Clone)]
struct StatusCardProps {
    label: &'static str,
    value: &'static str,
    detail: &'static str,
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
