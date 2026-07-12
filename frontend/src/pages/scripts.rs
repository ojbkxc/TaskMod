use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn ScriptsPage() -> Element {
    let mut scripts = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut refresh = use_signal(|| 0u32);
    let mut selected = use_signal(|| None::<String>);
    let mut script_content = use_signal(String::new);
    let mut editing = use_signal(|| false);

    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            error.set(None);
            match crate::api::client::get_scripts().await {
                Ok(list) => scripts.set(list),
                Err(e) => error.set(Some(format!("加载失败: {}", e))),
            }
            loading.set(false);
        });
    });

    let load_script = move |name: String| {
        let n = name.clone();
        selected.set(Some(name));
        spawn(async move {
            match crate::api::client::get_script_content(&n).await {
                Ok(content) => {
                    script_content.set(content);
                    editing.set(true);
                }
                Err(e) => error.set(Some(format!("加载脚本失败: {}", e))),
            }
        });
    };

    let save_script = move |_| {
        let name = selected.read().clone().unwrap_or_default();
        let content = script_content.read().clone();
        if name.is_empty() { return; }
        spawn(async move {
            match crate::api::client::save_script(&name, &content).await {
                Ok(_) => {
                    editing.set(false);
                    selected.set(None);
                    refresh += 1;
                }
                Err(e) => error.set(Some(format!("保存失败: {}", e))),
            }
        });
    };

    rsx! {
        div { class: "p-4 space-y-4",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "脚本管理" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "管理与运行自定义 Shell 脚本" }
                }
                div { class: "flex gap-1.5",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        onclick: move |_| refresh += 1,
                        "刷新"
                    }
                }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "p-2.5 rounded-md bg-[color-mix(in_srgb,var(--ds-error)_15%,transparent)] border border-[var(--ds-error)] text-[11px] text-[var(--ds-error)]",
                    "{err}"
                }
            }

            if *editing.read() {
                EqCard { class: "p-4",
                    div { class: "flex items-center justify-between mb-3",
                        span { class: "text-sm font-semibold text-[var(--ds-text)]",
                            "编辑: {selected}"
                        }
                        div { class: "flex gap-2",
                            EqButton {
                                variant: EqButtonVariant::Secondary,
                                onclick: move |_| { editing.set(false); selected.set(None); },
                                "取消"
                            }
                            EqButton {
                                variant: EqButtonVariant::Primary,
                                onclick: save_script,
                                "保存"
                            }
                        }
                    }
                    textarea {
                        class: "w-full min-h-[300px] px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-xs font-mono text-[var(--ds-text)] resize-y outline-none focus:border-[var(--ds-blue)]",
                        value: "{script_content}",
                        oninput: move |e| script_content.set(e.value.clone()),
                    }
                }
            }

            if *loading.read() && scripts.read().is_empty() {
                div { class: "text-center py-8 text-xs text-[var(--ds-text-tertiary)]", "加载中..." }
            }

            if scripts.read().is_empty() && !*loading.read() {
                div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2.5 p-6 text-center",
                    div { class: "flex items-center justify-center w-7 h-7 border border-[var(--ds-border)] rounded-md text-[var(--ds-text-tertiary)]",
                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" }
                        }
                    }
                    p { class: "text-sm font-semibold text-[var(--ds-text)]", "暂无脚本" }
                    p { class: "text-xs text-[var(--ds-text-tertiary)]", "脚本文件将在此列出" }
                }
            }

            div { class: "space-y-2",
                {scripts.read().iter().map(|name| {
                    let n = name.clone();
                    let n2 = name.clone();
                    rsx! {
                        EqCard { class: "p-3",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-2",
                                    svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10 20l4-16m4 4l4 4-4 4M6 16l-4-4 4-4" }
                                    }
                                    span { class: "text-sm font-medium text-[var(--ds-text)]", "n}" }
                                }
                                div { class: "flex gap-1",
                                    EqButton {
                                        variant: EqButtonVariant::Ghost,
                                        size: EqButtonSize::Sm,
                                        onclick: move |_| load_script(n2.clone()),
                                        "编辑"
                                    }
                                    EqButton {
                                        variant: EqButtonVariant::Ghost,
                                        size: EqButtonSize::Sm,
                                        onclick: move |_| {
                                            let nn = n.clone();
                                            spawn(async move {
                                                let _ = crate::api::client::delete_script(&nn).await;
                                                refresh += 1;
                                            });
                                        },
                                        "删除"
                                    }
                                }
                            }
                        }
                    }
                })}
            }
        }
    }
}
