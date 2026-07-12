use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn FilesPage() -> Element {
    let mut files = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut current_path = use_signal(|| "/".to_string());
    let mut refresh = use_signal(|| 0u32);

    let load_files = move |path: String| {
        let p = path.clone();
        current_path.set(path);
        spawn(async move {
            loading.set(true);
            error.set(None);
            match crate::api::client::list_files(&p).await {
                Ok(list) => files.set(list),
                Err(e) => error.set(Some(format!("加载失败: {}", e))),
            }
            loading.set(false);
        });
    };

    use_effect(move || {
        let _ = *refresh.read();
        let path = current_path.read().clone();
        load_files(path);
    });

    rsx! {
        div { class: "flex flex-col h-full",
            div { class: "flex items-start justify-between gap-3 p-4 pb-3 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "文件管理" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "浏览和管理设备文件" }
                }
            }

            div { class: "flex flex-1 min-h-0 overflow-hidden",
                div { class: "flex-1 flex flex-col min-w-0 overflow-hidden",
                    div { class: "flex items-center gap-1.5 p-2 bg-[var(--ds-card)] border-b border-[var(--ds-border)]",
                        EqButton {
                            variant: EqButtonVariant::Ghost,
                            size: EqButtonSize::Sm,
                            onclick: move |_| {
                                let cur = current_path.read().clone();
                                let parent = if cur == "/" { "/" } else {
                                    let trimmed = cur.trim_end_matches('/');
                                    match trimmed.rfind('/') {
                                        Some(pos) => &trimmed[..pos.max(1)],
                                        None => "/",
                                    }
                                };
                                load_files(parent.to_string());
                            },
                            "上级目录"
                        }
                        EqButton {
                            variant: EqButtonVariant::Ghost,
                            size: EqButtonSize::Sm,
                            onclick: move |_| refresh += 1,
                            "刷新"
                        }
                        div { class: "flex-1 px-2 text-xs text-[var(--ds-text-secondary)] font-mono",
                            "current_path}"
                        }
                    }

                    div { class: "flex-1 overflow-y-auto",
                        if *loading.read() {
                            div { class: "flex items-center justify-center min-h-[280px] text-sm text-[var(--ds-text-tertiary)]",
                                "加载中..."
                            }
                        } else if let Some(err) = error.read().as_ref() {
                            div { class: "p-4 text-xs text-[var(--ds-error)]", "{err}" }
                        } else if files.read().is_empty() {
                            div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2 text-center",
                                p { class: "text-sm text-[var(--ds-text-tertiary)]", "空目录" }
                            }
                        } else {
                            div { class: "divide-y divide-[var(--ds-border)]",
                                {files.read().iter().map(|f| {
                                    let name = f.get("name").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                                    let is_dir = f.get("is_dir").and_then(|v| v.as_bool()).unwrap_or(false);
                                    let size = f.get("size").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let display_name = name.clone();
                                    let path_for_click = if is_dir {
                                        let cur = current_path.read().clone();
                                        if cur.ends('/') { format!("{}{}", cur, name) } else { format!("{}/{}", cur, name) }
                                    } else { name.clone() };

                                    rsx! {
                                        div {
                                            class: "flex items-center gap-2 px-3 py-2 hover:bg-[var(--ds-surface-hover)] cursor-pointer text-sm",
                                            onclick: move |_| {
                                                if is_dir {
                                                    load_files(path_for_click.clone());
                                                }
                                            },
                                            if is_dir {
                                                svg { class: "w-4 h-4 text-[var(--ds-blue)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z" }
                                                }
                                            } else {
                                                svg { class: "w-4 h-4 text-[var(--ds-text-tertiary)]", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                                    path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z" }
                                                }
                                            }
                                            span { class: "flex-1 text-[var(--ds-text)]", "display_name}" }
                                            if !is_dir {
                                                span { class: "text-[10px] text-[var(--ds-text-tertiary)]", "{size} B" }
                                            }
                                        }
                                    }
                                })}
                            }
                        }
                    }

                    div { class: "flex items-center justify-between p-1.5 px-3 bg-[var(--ds-surface)] border-t border-[var(--ds-border)] text-[11px] text-[var(--ds-text-tertiary)]",
                        span { "就绪" }
                        span { "共 {files.read().len()} 个项目" }
                    }
                }
            }
        }
    }
}
