use dioxus::prelude::*;
use eq_ui::prelude::*;

/// 滚动到消息容器底部
pub fn scroll_to_bottom(container: &Signal<Option<MountedData>>) {
    if let Some(md) = container.read().as_ref() {
        if let Ok(element) = md.get() {
            if let Some(elem) = element.dyn_ref::<web_sys::HtmlDivElement>() {
                elem.set_scroll_top(elem.scroll_height());
            }
        }
    }
}

/// 简易 Markdown 渲染
pub fn render_markdown(content: &str) -> String {
    let mut result = content.to_string();
    result = result.replace("&", "&amp;");
    result = result.replace("<", "&lt;");
    result = result.replace(">", "&gt;");

    let mut parts = Vec::new();
    let mut lines = result.lines().peekable();

    while let Some(line) = lines.next() {
        if line.starts_with("```") {
            let lang = line[3..].trim().to_string();
            let mut code = String::new();
            while let Some(code_line) = lines.next() {
                if code_line.starts_with("```") {
                    break;
                }
                code.push_str(code_line);
                code.push('\n');
            }
            if !code.is_empty() {
                parts.push(format!(
                    "<pre class=\"bg-gray-800 text-gray-100 p-3 rounded-lg text-xs overflow-x-auto\"><code class=\"language-{}\">{}</code></pre>",
                    lang,
                    code.trim()
                ));
            }
        } else if line.starts_with("`") && line.ends_with("`") && line != "`" {
            parts.push(format!(
                "<code class=\"bg-gray-200 px-1.5 py-0.5 rounded text-xs font-mono\">{}</code>",
                &line[1..line.len() - 1]
            ));
        } else if line.starts_with("**") && line.ends_with("**") {
            parts.push(format!("<strong>{}</strong>", &line[2..line.len() - 2]));
        } else if line.starts_with("*") && line.ends_with("*") && line.len() > 2 {
            parts.push(format!("<em>{}</em>", &line[1..line.len() - 1]));
        } else if line.starts_with("# ") {
            parts.push(format!(
                "<h3 class=\"font-bold text-base mb-1\">{}</h3>",
                &line[2..]
            ));
        } else if line.starts_with("## ") {
            parts.push(format!(
                "<h4 class=\"font-semibold text-sm mb-1\">{}</h4>",
                &line[3..]
            ));
        } else if line.starts_with("- ") {
            parts.push(format!("<li class=\"ml-4 text-sm\">{}</li>", &line[2..]));
        } else if line.starts_with("1. ") {
            parts.push(format!("<li class=\"ml-4 text-sm\">{}</li>", &line[3..]));
        } else if line.starts_with("> ") {
            parts.push(format!(
                "<blockquote class=\"border-l-2 border-gray-300 pl-3 italic text-sm text-gray-600\">{}</blockquote>",
                &line[2..]
            ));
        } else {
            parts.push(line.to_string());
        }
    }

    parts.join("\n")
}

#[derive(Props, PartialEq, Clone)]
pub struct QuickPromptCardProps {
    pub label: &'static str,
    pub on_click: EventHandler<()>,
}

#[component]
pub fn QuickPromptCard(props: QuickPromptCardProps) -> Element {
    rsx! {
        button {
            class: "flex items-center justify-center gap-2 px-3 py-2.5 border border-[var(--ds-border)] rounded-lg bg-[var(--ds-card)] text-[var(--ds-text-secondary)] cursor-pointer text-xs font-medium transition-all hover:border-[var(--ds-blue)] hover:bg-[var(--ds-blue-light)] hover:text-[var(--ds-blue)]",
            onclick: props.on_click,
            "{props.label}"
        }
    }
}