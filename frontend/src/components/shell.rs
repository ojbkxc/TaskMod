use dioxus::prelude::*;
use eq_ui::prelude::*;

use crate::app::ActivePage;
use crate::components::sidebar::Sidebar;

#[derive(Props, PartialEq, Clone)]
pub struct AppShellProps {
    pub theme: Signal<String>,
    pub active_page: Signal<ActivePage>,
}

#[component]
pub fn AppShell(props: AppShellProps) -> Element {
    rsx! {
        div { class: "flex flex-col h-screen min-w-[320px]",
            Sidebar {
                theme: props.theme,
                active_page: props.active_page,
            }
        }
    }
}
