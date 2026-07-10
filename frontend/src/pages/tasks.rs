use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn TasksPage() -> Element {
    rsx! {
        div { class: "p-4 space-y-4",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "任务管理" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "管理定时任务与命令" }
                }
                div { class: "flex gap-1.5",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        "刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        "添加任务"
                    }
                }
            }

            // 空状态
            div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2.5 p-6 text-center",
                div { class: "flex items-center justify-center w-7 h-7 border border-[var(--ds-border)] rounded-md text-[var(--ds-text-tertiary)]",
                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" }
                    }
                }
                p { class: "text-sm font-semibold text-[var(--ds-text)]", "暂无任务" }
                p { class: "text-xs text-[var(--ds-text-tertiary)]", "点击\"添加任务\"创建定时任务" }
            }
        }
    }
}
