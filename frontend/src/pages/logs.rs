use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn LogsPage() -> Element {
    let mut logs = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut auto_refresh = use_signal(|| true);
    let mut refresh = use_signal(|| 0u32);

    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            match crate::api::client::get_logs(200).await {
                Ok(list) => logs.set(list),
                Err(_) => {}
            }
            loading.set(false);
        });
    });

    // 自动刷新定时器
    use_effect(move || {
        if *auto_refresh.read() {
            spawn(async move {
                loop {
                    gloo_timers::future::TimeoutFuture::new(3000).await;
                    if !*auto_refresh.read() { break; }
                    match crate::api::client::get_logs(200).await {
                        Ok(list) => logs.set(list),
                        Err(_) => {}
                    }
                }
            });
        }
    });

    rsx! {
        div { class: "p-4 space-y-4",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "日志" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "查看系统运行日志" }
                }
                div { class: "flex gap-1.5 items-center",
                    label { class: "flex items-center gap-1 text-xs cursor-pointer",
                        input {
                            r#type: "checkbox",
                            checked: *auto_refresh.read(),
                            onchange: move |e| auto_refresh.set(e.checked()),
                        }
                        "自动刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        onclick: move |_| refresh += 1,
                        "刷新"
                    }
                    EqButton {
                        variant: EqButtonVariant::Destructive,
                        onclick: move |_| {
                            spawn(async move {
                                let _ = crate::api::client::clear_logs().await;
                                refresh += 1;
                            });
                        },
                        "清除"
                    }
                }
            }

            div { class: "bg-[var(--ds-bg)] border border-[var(--ds-border)] rounded-md p-2.5 font-mono text-[11px] leading-relaxed whitespace-pre-wrap break-all max-h-[500px] overflow-y-auto",
                if *loading.read() && logs.read().is_empty() {
                    div { class: "text-center py-8 text-xs text-[var(--ds-text-tertiary)]", "加载中..." }
                } else if logs.read().is_empty() {
                    div { class: "flex flex-col items-center justify-center min-h-[280px] gap-2.5 p-6 text-center",
                        p { class: "text-sm font-semibold text-[var(--ds-text)]", "暂无日志" }
                        p { class: "text-xs text-[var(--ds-text-tertiary)]", "系统日志将在此显示" }
                    }
                } else {
                    {logs.read().iter().map(|line| rsx! {
                        div { class: "py-0.5 border-b border-[var(--ds-border)] last:border-b-0",
                            "{line}"
                        }
                    })}
                }
            }
        }
    }
}
