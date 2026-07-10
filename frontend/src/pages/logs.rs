use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn LogsPage() -> Element {
    rsx! {
        div { class: "p-4 space-y-4",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "日志" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "查看系统运行日志" }
                }
                div { class: "flex gap-1.5 items-center",
                    label { class: "flex items-center gap-1 text-xs cursor-pointer",
                        input { type: "checkbox" }
                        "自动刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Destructive,
                        "清除"
                    }
                }
            }

            // 日志查看器
            div { class: "bg-[var(--ds-bg)] border border-[var(--ds-border)] rounded-md p-2.5 font-mono text-[11px] leading-relaxed whitespace-pre-wrap break-all max-h-[400px] overflow-y-auto",
                div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2.5 p-6 text-center",
                    div { class: "flex items-center justify-center w-7 h-7 border border-[var(--ds-border)] rounded-md text-[var(--ds-text-tertiary)]",
                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" }
                        }
                    }
                    p { class: "text-sm font-semibold text-[var(--ds-text)]", "暂无日志" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)]", "系统日志将在此显示" }
                }
            }
        }
    }
}
