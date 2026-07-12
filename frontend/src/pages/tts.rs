use dioxus::prelude::*;
use eq_ui::prelude::*;

#[component]
pub fn TtsPage() -> Element {
    let mut engines = use_signal(Vec::new);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut selected_engine = use_signal(|| None::<String>);
    let mut text = use_signal(String::new);
    let mut status_msg = use_signal(|| None::<String>);
    let mut refresh = use_signal(|| 0u32);

    use_effect(move || {
        let _ = *refresh.read();
        spawn(async move {
            loading.set(true);
            match crate::api::client::get_tts_engines().await {
                Ok(list) => {
                    if list.is_empty() {
                        selected_engine.set(None);
                    } else {
                        let first_name = list[0].get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if selected_engine.read().is_none() {
                            selected_engine.set(Some(first_name));
                        }
                    }
                    engines.set(list);
                }
                Err(e) => error.set(Some(format!("加载引擎失败: {}", e))),
            }
            loading.set(false);
        });
    });

    rsx! {
        div { class: "p-4 space-y-4",
            div { class: "flex items-start justify-between gap-3 pb-4 border-b border-[var(--ds-border)]",
                div {
                    h1 { class: "text-lg font-bold text-[var(--ds-text)]", "TTS 语音合成" }
                    p { class: "text-xs text-[var(--ds-text-secondary)] mt-2", "文本转语音控制面板" }
                }
                div { class: "flex gap-1.5",
                    EqButton {
                        variant: EqButtonVariant::Secondary,
                        onclick: move |_| refresh += 1,
                        "刷新引擎"
                    }
                }
            }

            if let Some(err) = error.read().as_ref() {
                div { class: "p-2.5 rounded-md bg-[color-mix(in_srgb,var(--ds-error)_15%,transparent)] border border-[var(--ds-error)] text-[11px] text-[var(--ds-error)]",
                    "err}"
                }
            }

            EqCard { class: "p-4",
                div { class: "flex items-center justify-between mb-3",
                    span { class: "text-sm font-semibold text-[var(--ds-text)] flex items-center gap-1.5",
                        "语音引擎"
                    }
                }
                div { class: "flex gap-2 items-center",
                    select {
                        class: "flex-1 min-h-[42px] px-3 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)]",
                        onchange: move |e| selected_engine.set(Some(e.value())),
                        if engines.read().is_empty() {
                            option { "无可用引擎" }
                        } else {
                            {engines.read().iter().map(|eng| {
                                let name = eng.get("name").and_then(|v| v.as_str()).unwrap_or("未知");
                                rsx! {
                                    option { value: "{name}", "{name}" }
                                }
                            })}
                        }
                    }
                }
            }

            EqCard { class: "p-4",
                div { class: "flex items-center justify-between mb-3",
                    span { class: "text-sm font-semibold text-[var(--ds-text)]", "文本输入" }
                }
                textarea {
                    class: "w-full min-h-[100px] px-3 py-2 border border-[var(--ds-border)] rounded-md bg-[var(--ds-bg)] text-sm text-[var(--ds-text)] resize-y outline-none focus:border-[var(--ds-blue)]",
                    placeholder: "输入要朗读的文本...",
                    value: "text}",
                    oninput: move |e| text.set(e.value.clone()),
                }
                div { class: "flex items-center justify-between mt-2",
                    span { class: "text-[11px] text-[var(--ds-text-tertiary)]",
                        "{text.read().len()} 字"
                    }
                    div { class: "flex gap-2",
                        EqButton {
                            variant: EqButtonVariant::Primary,
                            onclick: move |_| {
                                let t = text.read().clone();
                                let eng = selected_engine.read().clone();
                                if t.is_empty() { return; }
                                spawn(async move {
                                    match crate::api::client::tts_speak(&t, eng.as_deref()).await {
                                        Ok(msg) => status_msg.set(Some(msg)),
                                        Err(e) => status_msg.set(Some(format!("失败: {}", e))),
                                    }
                                });
                            },
                            "朗读"
                        }
                        EqButton {
                            variant: EqButtonVariant::Destructive,
                            onclick: move |_| {
                                spawn(async move {
                                    match crate::api::client::tts_stop().await {
                                        Ok(msg) => status_msg.set(Some(msg)),
                                        Err(e) => status_msg.set(Some(format!("失败: {}", e))),
                                    }
                                });
                            },
                            "停止"
                        }
                    }
                }
            }

            if let Some(msg) = status_msg.read().as_ref() {
                div { class: "text-xs text-[var(--ds-text-secondary)] p-2 bg-[var(--ds-surface)] rounded",
                    "{msg}"
                }
            }
        }
    }
}
