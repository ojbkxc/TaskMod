use dioxus::prelude::*;
use eq_ui::prelude::*;

use crate::components::shell::AppShell;
use crate::pages::dashboard::DashboardPage;
use crate::pages::chat::ChatPage;
use crate::pages::daemon::DaemonPage;
use crate::pages::tasks::TasksPage;
use crate::pages::scripts::ScriptsPage;
use crate::pages::mirror::MirrorPage;
use crate::pages::files::FilesPage;
use crate::pages::tts::TtsPage;
use crate::pages::config::ConfigPage;
use crate::pages::logs::LogsPage;
use crate::pages::library::LibraryPage;

/// 当前活跃的页面
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActivePage {
    Dashboard,
    Chat,
    Daemon,
    Mirror,
    Library,
    Tasks,
    Scripts,
    Files,
    Tts,
    Config,
    Logs,
}

impl ActivePage {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "仪表盘",
            Self::Chat => "AI助手",
            Self::Daemon => "隧道",
            Self::Mirror => "设备",
            Self::Library => "知识库",
            Self::Tasks => "任务",
            Self::Scripts => "脚本",
            Self::Files => "文件",
            Self::Tts => "TTS",
            Self::Config => "配置",
            Self::Logs => "日志",
        }
    }
}

#[component]
pub fn App() -> Element {
    // 主题状态：light / dark
    let mut theme = use_signal(|| "dark".to_string());
    // 当前页面
    let mut active_page = use_signal(|| ActivePage::Dashboard);

    rsx! {
        EqThemeProvider { theme: theme(),
            EqAppShell {
                // Header 区域：包含侧边栏导航
                header: rsx! {
                    AppShell {
                        theme: theme,
                        active_page: active_page,
                    }
                },
                // Main 区域：根据 active_page 渲染对应页面
                main: rsx! {
                    div { class: "flex-1 overflow-y-auto bg-[var(--ds-bg)]",
                        match *active_page.read() {
                            ActivePage::Dashboard => rsx! { DashboardPage {} },
                            ActivePage::Chat => rsx! { ChatPage {} },
                            ActivePage::Daemon => rsx! { DaemonPage {} },
                            ActivePage::Mirror => rsx! { MirrorPage {} },
                            ActivePage::Library => rsx! { LibraryPage {} },
                            ActivePage::Tasks => rsx! { TasksPage {} },
                            ActivePage::Scripts => rsx! { ScriptsPage {} },
                            ActivePage::Files => rsx! { FilesPage {} },
                            ActivePage::Tts => rsx! { TtsPage {} },
                            ActivePage::Config => rsx! { ConfigPage {} },
                            ActivePage::Logs => rsx! { LogsPage {} },
                        }
                    }
                },
            }
        }
    }
}
