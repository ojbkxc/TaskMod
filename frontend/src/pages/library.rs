use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn LibraryPage() -> Element {
    rsx! {
        div { class: "p-4 space-y-4",
            // 页面标题
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "知识库" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "管理记忆、预设和技能" }
                }
            }

            // 子标签页
            div { class: "flex gap-1 overflow-x-auto pb-3 border-b border-[var(--ds-border)]",
                EqTab { active: true, "记忆" }
                EqTab { "预设" }
                EqTab { "技能" }
                EqTab { "保存项" }
                EqTab { "场景" }
                EqTab { "项目" }
                EqTab { "MCP" }
                EqTab { "截图" }
                EqTab { "Prompt控制" }
            }

            // 内容区域
            div { class: "space-y-3",
                // 搜索和新建
                div { class: "flex gap-2",
                    input {
                        class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] outline-none focus:border-[var(--ds-blue)]",
                        placeholder: "搜索记忆...",
                    }
                    EqButton {
                        variant: EqButtonVariant::Primary,
                        "新建"
                    }
                }
            }
        }
    }
}
