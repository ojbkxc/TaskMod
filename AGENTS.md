# AGENTS.md — TaskMod 项目代理工作指引

> 本文件供 AI 编码代理（含未来会话）进入项目时**首先自读**，快速对齐项目定位、当前进度、架构契约与下一步任务，然后**继续完善未完成的代码**。
> 优先级：本文件 > `README.md` > `CLAUDE.md`（UI 设计规范）。

---

## R0. 强制规则（MANDATORY，不可绕过）

> 本节为**最高优先级的强制约束**，凌驾于一切其他指引之上。违反即视为流程失败。

1. **每次会话必须先自读本文件**：进入项目后，在执行任何写代码/搜索/构建动作之前，必须先 `read` 完整 `AGENTS.md`，对齐「当前进度」「下一步任务」「接口契约」。
2. **每次会话结束前必须回写本文件**：无论本次完成了几项任务（含 0 项，即仅排查/失败），在结束前**必须**用 `edit`/`write` 更新本文件至少一处：
   - **必须**更新「§9 变更日志」追加一行（最新在上），记录本次做了什么、改了哪些文件、是否通过验证、下一步建议。
   - **必须**更新「§4 当前进度」与「§6 下一步任务」的勾选状态以反映真实状态（新完成的挪到「已完成」区，新发现的问题加入「已知小问题」）。
   - 若改动了接口契约，**必须**同步更新「§5 关键接口契约」。
   - 若改动了目录结构或新增/删除文件，**必须**同步更新「§3 仓库结构」。
3. **本文件是单一事实源（single source of truth）**：当本文件与代码、与 `README.md`、与口头描述出现矛盾时，**先以代码为准**，然后**立即回写本文件**消除漂移；禁止让本文件与代码长期不一致。
4. **不得删除或弱化本节**：任何对「§R0 强制规则」的删减、降级、加「视情况而定」修饰，都需用户明确同意；代理自身不得自行放宽。
5. **跟进是义务而非可选**：即使用户未要求「更新 AGENTS.md」，每次会话结束前也必须执行回写；用户明确说「不用更新」时才可跳过，并在变更日志注明「依用户要求跳过本次回写」。
6. **编译验证必须提交到 GitHub 上编译**（MANDATORY）：本地为离线环境，缺 Rust 交叉编译工具链 / Android NDK / Dioxus CLI，**无法**本地完整构建。因此**任何代码改动后的编译验证必须通过 `git commit && git push` 提交到 GitHub**（`origin = https://github.com/ojbkxc/TaskMod.git`，分支 `main`），由 GitHub CI（`.github/workflows/`，见 §R2）执行构建。**禁止**在未 push 到 GitHub 编译通过前声称某子任务「完成/已验证」。
7. **通过 GitHub 编译报错迭代修复**（MANDATORY）：push 后若 GitHub CI 编译/测试失败，**必须**读取 CI 日志中的报错，据报错本地修复后**再次 commit & push**，循环直至 CI 全绿。**不得**跳过 CI 失败直接推进下一子任务；**不得**用 `#[allow(...)]`/注释掉测试/降低 clippy 阈值等方式绕过 CI 报错（除非用户明确同意）。CI 全绿是子任务完成的**唯一**编译验证判据。
8. **自动推进项目（auto-continue，默认行为）**（MANDATORY）：用户说「自动继续」/「继续」/「auto」或未明确叫停时，代理**必须自主连续推进**项目任务，不得每完成一小步就停下来询问下一步。具体要求：
    - 进入项目后按 §0 流程**自主**挑选下一个最高优先级的最小可独立交付子任务并开工，不等用户逐项指派。
    - 单个子任务完成后**立即**开始下一个，无需请求许可；仅在遇到「方向性分歧」「破坏性操作」「违反硬约束」「信息严重不足且无法合理推断」时才用 `question` 工具询问用户。
    - 推进过程中**主动**走 §R2.3 CI 修复闭环、§R0 回写，不要等用户提醒。
    - 用户未说「自动继续」时也鼓励减少不必要的中途提问，但可在阶段切换时简要汇报进度；用户说「自动继续」后则**连续作业**直到任务全部完成或遇阻才停下汇报。
    - 停下汇报时应附「已完成的 / 正在做的 / 下一步打算做的」三段式摘要，便于用户一句话继续（如「继续」「换方向」「停」）。
9. **UI 设计遵循 CLAUDE.md**：前端 UI 改动必须遵循 `CLAUDE.md` 的 DeepSeek++ 设计语言（克制耐看、minimal 主题、深色/浅色双模式、无 Emoji 堆砌、eq_ui 原子设计）。

---

## R2. GitHub CI 编译验证策略（MANDATORY，配合 §R0.6–R0.7）

> 本节落实 §R0.6/R0.7 的「提交到 GitHub 编译 + 据报错修复」闭环。本地离线不可编译，GitHub CI 是**唯一**编译验证通道。

### R2.1 CI 触发条件
- **`ci.yml`**：push 到 `main` 或 PR 时触发，执行服务端交叉编译 + APK 构建 + Clippy + fmt 检查（不发版）。
- **`build.yml`**：push tag `v*`（如 `v1.0.14`）或手动 `workflow_dispatch` 触发，执行完整构建 + GitHub Release 发版。
- **`build-apk.yml`**：push tag `v*` 且改动 `android/**` 时触发，独立构建 APK Release。
- CI 在 GitHub-hosted runner（ubuntu-latest，可联网拉工具链/依赖）上执行，规避本地离线缺工具链问题。

### R2.2 CI 必须执行的步骤（全绿才算通过）
```
# .github/workflows/ci.yml（PR/push 检查）
1. build-server job:
   - checkout
   - Install Rust + target aarch64-linux-android
   - Setup Android NDK r27c
   - cargo ndk -t arm64-v8a build --release  # 服务端交叉编译
   - 打包 Magisk 模块 zip
2. build-apk job:
   - JDK 17 + Gradle 8.5
   - ./gradlew assembleDebug  # APK 构建
3. lint job:
   - cargo clippy -- -D warnings  # Rust lint（警告即失败）
   - cargo fmt --check            # Rust 格式检查

# .github/workflows/build.yml（tag/workflow_dispatch 发版）
1. build-server: 同上 + 更新 module.prop/Cargo.toml 版本号
2. build-apk: JDK 17 + ./gradlew assembleDebug
3. release: 上传 TaskMod-{VERSION}.zip + TaskMod-app-{VERSION}.apk 到 GitHub Release
```

### R2.3 据报错修复的迭代流程（每次 push 后必走）
1. `git push origin main`（或 `git push origin v1.0.14` 触发发版）。
2. 用 `gh run watch` 或浏览器查看 `https://github.com/ojbkxc/TaskMod/actions` 的运行结果。
3. 若失败：`gh run view --log-failed` 取报错日志，定位首个 `error:` / `FAILED` / `warning:` 行。
4. 本地按报错修代码（修 import/类型/借用/生命周期/资源引用等），**不**绕过（不 `#[allow]`、不删测试、不降低 clippy 阈值）。
5. `git commit && git push`，回到步骤 2，直至 CI 全绿。
6. CI 全绿后才能在 §4/§6 勾选该子任务「完成」并在 §9 变更日志注明「CI 全绿验证通过」。

### R2.4 本地可做的静态检查（push 前自检，减少 CI 往返）
- **`git status` 确认无残留未 commit 修改**：会话开始前和 commit 前各执行一次，确保所有修改的文件都被 staged。**这是最常见的 CI 失败根因之一**——修改了文件但忘记 commit，CI 用的是旧版本。
- **Rust 静态检查（本地有 Rust 工具链时）**：
  - `cargo check`（server 目录）：类型/借用检查，不生成产物。
  - `cargo clippy -- -D warnings`：lint 检查（CI 会执行，本地先跑可提前发现问题）。
  - `cargo fmt --check`：格式检查（CI 会执行）。
- **Kotlin 静态检查（本地无法编译，必须人工查）**：
  - import 路径、nullable 类型（`String?` vs `String`）、`suspend` 函数/lambda 类型匹配。
  - 新增参数：确认所有调用点都传了正确类型的参数。
  - 确认无 `com.lxseek.chat`（Agora 包名）误引入——TaskMod 包名是 `com.taskmod.app`。
- **前端检查（本地有 Dioxus CLI 时）**：
  - `dx check`（frontend 目录）：Dioxus 0.7 类型检查。
  - 确认 `eq_ui` 组件用法符合 0.5 版本 API。
- **版本号同步**：发版前确认 `server/Cargo.toml` 的 `version` 与 `module.prop` 的 `version`/`versionCode` 与 `android/app/build.gradle` 的 `versionName`/`versionCode` 三处一致（CI 会自动更新，但手动改时要同步）。

### R2.5 CI workflow 维护
- 若新增 Rust 依赖或改变 NDK 版本/ABI/Gradle 版本，同步更新 `.github/workflows/*.yml` 与 `server/Cargo.toml` / `android/app/build.gradle`。
- 若新增 signing secret，在 GitHub repo Settings → Secrets 配置后更新 workflow 的 `env` 映射。

---

## 0. 进入项目后的标准流程（必读）

1. **通读本文件**（尤其是「§R0 强制规则」「当前进度」「下一步任务」「编码约定」五节）。
1b. **`git status` 检查残留修改**：若工作目录有未 commit 的修改（来自前次会话遗漏），先理解其内容并 commit，再开始新工作。**不要**在新工作开始前 `git stash` 或 `git checkout -- .` 丢弃前次修改——先搞清楚是什么、是否需要保留。
2. 按「下一步任务」的优先级顺序挑选一个**最小可独立交付**的子任务开工。
3. 开工前用 `read`/`grep`/`glob` 阅读相关已有代码；**复用既有 API handler、工具函数、组件与命名**，不要另起炉灶。
4. 每完成一个子任务：执行 §R2.4 静态检查清单，然后 `git add -A && git status` 确认所有修改已 staged，`git commit && git push` 触发 CI 验证。
5. **回写本文件**（强制，见 §R0）：更新「当前进度」「下一步任务」勾选状态，并在「变更日志」追加一行。
6. **不要**主动 `git commit`，除非用户明确要求。**不要**写未经请求的 README/文档。**不要**加注释除非用户要求。
7. **会话结束前再次确认 §R0 的回写已执行**；若未执行，补做后再结束。

---

## 1. 项目定位（一句话）

TaskMod 是 **基于 Rust 的 Android 设备自动化管理工具** — 服务端（Rust + axum）跑在设备上，Web 前端（Dioxus 0.7 → WASM）+ Android APK（Kotlin）+ Magisk 模块三形态分发。支持屏幕镜像、AI 控制（多供应商 + 工具调用）、脚本执行、TTS 语音、MQTT、邮件通知、工作流引擎。MIT 许可证。

## 2. 硬约束（任何改动都不得违反）

| 维度 | 约束 | 验证方式 |
|---|---|---|
| 仓库 | `https://github.com/ojbkxc/TaskMod.git` | `git remote -v` |
| 服务端 target | **仅 `aarch64-linux-android`**（arm64-v8a） | `cargo ndk -t arm64-v8a` |
| 前端 target | **`wasm32-unknown-unknown`** | `dx build --release` |
| Rust edition | 2021 | `server/Cargo.toml` / `frontend/Cargo.toml` |
| NDK | r27c | `.github/workflows/*.yml` |
| Android SDK | minSdk 21 / targetSdk 34 / compileSdk 34 | `android/app/build.gradle` |
| JDK | 17 | `.github/workflows/*.yml` |
| Gradle | 8.5 | `.github/workflows/*.yml` |
| Kotlin | org.jetbrains.kotlin.android（build.gradle 插件） | `android/app/build.gradle` |
| Dioxus | 0.7 | `frontend/Cargo.toml` |
| eq_ui | 0.5 | `frontend/Cargo.toml` |
| APK applicationId | `com.taskmod.app` | `android/app/build.gradle` |
| APK namespace | `com.taskmod.app` | `android/app/build.gradle` |
| 服务端口 | 9527（默认，可在 APK 设置中改） | `server/src/config.rs` |
| 版本 | 1.0.15 / versionCode 1000015 | `server/Cargo.toml` + `module.prop` + `android/app/build.gradle` |
| 产物命名 | `TaskMod-{VERSION}.zip`（Magisk）+ `TaskMod-app-{VERSION}.apk`（APK） | CI `build.yml` |
| 许可证 | MIT | `LICENSE` |
| UI 设计 | DeepSeek++ 风格（CLAUDE.md） | `CLAUDE.md` |

新增 Rust 依赖前先评估对二进制体积的影响（release profile 已设 `opt-level = "z"` + `lto = true` + `codegen-units = 1` + `strip = true` 极致压缩）；优先用 `rustls-tls` 避免 OpenSSL C 依赖（`reqwest` 已用 rustls，但 `openssl-sys` vendored 仍保留用于交叉编译兼容）。

## 3. 仓库结构与模块划分

```
TaskMod/
├── AGENTS.md                          # 本文件（代理工作指引）
├── CLAUDE.md                          # UI 设计规范（DeepSeek++ 风格）
├── README.md                          # 项目说明
├── module.prop                        # Magisk 模块属性（id/name/version/versionCode）
├── customize.sh                       # Magisk 安装脚本
├── service.sh                         # Magisk 服务启动脚本
├── META-INF/                          # Magisk 模块配置
├── sdcard/TaskMod/                    # 用户数据目录（APK 与模块共享）
│   ├── app_settings.json              # APK 统一配置（端口/IP/域名/自启动）
│   ├── scripts/                       # 脚本目录
│   ├── screenshots/                   # 截图目录
│   ├── workflows/                     # 工作流目录
│   ├── chat_history/                  # AI 对话历史
│   ├── memory/                        # AI 记忆系统
│   ├── skills/                        # AI Skill 文件（热加载）
│   ├── saved_items/                   # AI 保存项
│   ├── projects/                      # AI 项目上下文
│   ├── mcp/                           # MCP 服务器配置（热加载）
│   ├── presets.json                   # Prompt 预设
│   ├── prompt_settings.json           # Prompt 注入设置
│   ├── scenarios.json                 # 场景模板
│   ├── schedule.conf                  # 定时任务配置
│   ├── email.conf                     # 邮件配置
│   ├── ai.conf                        # AI 供应商配置
│   └── mqtt.conf                      # MQTT 配置（可选）
├── server/                            # Rust 服务端源码（35 个 .rs 文件）
│   ├── Cargo.toml                     # 依赖：axum 0.6 + tokio + reqwest(rustls) + openssl-sys(vendored)
│   ├── Cargo.lock
│   └── src/
│       ├── main.rs                    # 入口（路由注册 + 看门狗 + 事件监控 + MQTT + UDP 发现）
│       ├── config.rs                  # 路径常量 + 端口读取
│       ├── state.rs                   # MirrorState（投屏状态）
│       ├── kcp_stream.rs              # KCP 可靠 UDP 传输
│       ├── platform.rs                # 平台适配
│       ├── api/                       # API 路由（10 个 .rs 文件）
│       │   ├── ai.rs                  # AI 对话核心（build_api_url 智能URL拼接 + 流式SSE）
│       │   ├── ai_hub.rs              # AI Hub（会话/预设/记忆/Skill/场景/保存项/项目/MCP）
│       │   ├── mirror.rs              # 投屏控制（H.264 + 音频采集 + KCP）
│       │   ├── daemon.rs              # 隧道与服务管理
│       │   ├── system.rs              # 系统管理（截图/配置/邮件/工作流/事件）
│       │   ├── tasks.rs               # 定时任务
│       │   ├── scripts.rs             # 脚本管理
│       │   ├── tts.rs                 # 语音播报（优先 APK 广播 → fallback shell）
│       │   └── ...                    # 其他 API
│       ├── data/                      # 数据模型 + 配置
│       │   ├── tts_config.rs          # TTS 配置（引擎参数/替换规则/分句）
│       │   ├── response.rs            # ApiResponse 统一响应
│       │   ├── models.rs              # 数据模型
│       │   └── ...
│       ├── tools/                     # AI Tool Calling（ADB/脚本/任务/TTS 工具）
│       ├── utils/                     # 工具模块（email/mqtt/event_monitor 等）
│       └── ...
├── frontend/                          # Web 前端源码（Dioxus 0.7 → WASM）
│   ├── Cargo.toml                     # 依赖：dioxus 0.7 + eq_ui 0.5 + web-sys
│   └── src/
│       ├── main.rs                    # 入口
│       ├── pages/                     # 页面组件
│       │   ├── dashboard.rs           # 仪表盘
│       │   ├── chat.rs                # AI 对话
│       │   ├── mirror.rs              # 投屏（左右三栏 + Web Audio API）
│       │   ├── config.rs              # 配置（AI/邮件/MQTT/语音）
│       │   ├── daemon.rs              # 隧道管理
│       │   ├── chat/                  # 对话子组件
│       │   └── ...
│       ├── components/                # 公共组件（EqCard/EqButton 等）
│       └── api/                       # API 客户端
├── android/                           # Android APK 源码（Kotlin）
│   ├── build.gradle                   # 顶层构建
│   ├── settings.gradle
│   └── app/
│       ├── build.gradle               # 应用构建（applicationId com.taskmod.app / minSdk 21 / targetSdk 34）
│       └── src/main/
│           ├── AndroidManifest.xml    # 权限 + 组件声明（含 QUERY_ALL_PACKAGES + <queries> TTS_SERVICE）
│           ├── assets/                # taskmod-server 二进制（CI 注入）
│           ├── java/com/taskmod/app/
│           │   ├── TaskModApp.kt      # Application（通知渠道 + ConfigManager + TtsManager.init）
│           │   ├── MainActivity.kt    # 主界面
│           │   ├── WebViewActivity.kt # 内嵌浏览器
│           │   ├── SettingsActivity.kt# 设置页
│           │   ├── ConfigManager.kt   # 统一配置（/sdcard/TaskMod/app_settings.json）
│           │   ├── NetworkHelper.kt   # 多网卡 IP 检测
│           │   ├── ServerManager.kt   # 服务进程管理
│           │   ├── TaskModService.kt  # 前台服务
│           │   ├── RootHelper.kt      # Root 命令执行
│           │   ├── TtsManager.kt      # 原生 TTS（借鉴 Agora，多引擎+看门狗+诊断）
│           │   ├── TtsReceiver.kt     # TTS 广播接收器（供 server am broadcast 调用）
│           │   ├── DaemonManager.kt   # 隧道管理
│           │   ├── UpdateChecker.kt   # 自动更新
│           │   ├── BootReceiver.kt    # 开机启动
│           │   ├── UpdateReceiver.kt  # 更新下载完成
│           │   ├── MagiskGuideActivity.kt # 模块安装引导
│           │   ├── widget/            # 桌面小组件
│           │   └── tiles/             # Quick Settings 磁贴
│           └── res/                   # 资源（drawable/values/values-night/xml）
├── bin/                               # 编译产物目录（CI 注入 arm64/taskmod-server）
└── .github/workflows/
    ├── build.yml                      # CI/CD: 服务端 + APK + Magisk 模块 + Release
    ├── build-apk.yml                  # APK 独立构建（含 server 注入 assets）
    └── ci.yml                         # PR/push 检查（编译 + Clippy + fmt）
```

**数据流**：
- **设备控制**：`Web 前端 → axum API → ADB/shell 命令 → Android 系统`
- **AI 对话**：`Web 前端 ↔ WebSocket ↔ axum ai.rs → reqwest → LLM 供应商 → 流式 SSE 回传`；工具调用经 `tools/`（ADB/脚本/任务/TTS）
- **投屏**：`Android screencap/H.264 → axum mirror.rs → KCP/WebSocket → Web 前端 Web Audio API`
- **TTS**：`axum tts.rs → am broadcast → APK TtsReceiver → TtsManager → Android TextToSpeech`（主路径）；fallback `am startservice / cmd tts speak`（shell）
- **热加载服务**：MQTT/邮件/TTS 按需加载，未配置时零内存占用

## 4. 当前进度（截至 2026-08-17）

### ✅ 已完成
- **v1.0.15 发版**：bump 三处版本号到 1.0.15 / versionCode 1000015（`server/Cargo.toml` + `module.prop` + `android/app/build.gradle`），commit `eb5dd70`，tag `v1.0.15` 已 push。Build & Release (run 32014268222) **success**，Release v1.0.15 已创建：TaskMod-1.0.15.zip (5.38 MB) + TaskMod-app-1.0.15.apk (7.09 MB)。CI (run 32014263189) success。附带修复 build-apk.yml（原手动 config.toml + cargo build 致 ring/openssl-sys 编译失败，改用 cargo-ndk 与 build.yml 一致，commit `c506ab7`，workflow_dispatch run 32015753734 success）。
- **v1.0.14 TTS 原生调用借鉴 Agora**：用户反馈 TaskMod 的 shell 命令调用 TTS 失败，Agora 调用系统 TTS 成功。借鉴 Agora 的 `TtsManager.kt` 到 TaskMod APK 端：新建 `TtsManager.kt`（333 行，多引擎切换+看门狗 30s+诊断日志+init 重试+stale 回调防护+stripMarkdown+setLanguage 三级回退+主线程 speak+setPitch）+ `TtsReceiver.kt`（54 行，BroadcastReceiver 接收 TTS_SPEAK/TTS_STOP/TTS_INIT）；修改 `AndroidManifest.xml`（注册 TtsReceiver + `<queries>` 声明 TTS_SERVICE）+ `TaskModApp.kt`（onCreate 调 TtsManager.init）；修改 `server/src/api/tts.rs`（新增 `exec_speak_via_apk` 函数，`exec_speak` 优先 `am broadcast` 调 APK，失败 fallback shell 命令）。CI 全绿验证通过（run 32013772236, commit 46a6e54）。

### 🔲 未开始 / 进行中
- （暂无明确未完成任务，按用户需求推进）

### ⚠️ 已知小问题
- `server/Cargo.toml` 的 `openssl-sys = { features = ["vendored"] }` 强制从源码编译 OpenSSL，本地 `cargo check` 需要 perl，Windows 环境可能失败。CI（ubuntu-latest）正常。本地静态检查可跳过完整编译，用 `cargo check` 不触发 openssl build script 的方式或直接 push 到 CI 验证。
- 前端 `frontend/` 的 Dioxus 0.7 + eq_ui 0.5 在本地无 `dx` CLI 时无法编译，需 push 到 CI 或安装 Dioxus 工具链。

## 5. 关键接口契约（不要破坏既有签名）

### 5.1 axum API 路由（`server/src/main.rs`）
- 所有 API 统一返回 `Json<ApiResponse<T>>`（`data/response.rs`），`ApiResponse::ok(data)` / `ApiResponse::err(msg)` / `ApiResponse::ok_msg(data, msg)`。
- TTS 路由（`api/tts.rs`）：
  - `GET  /api/tts/engines` → `Vec<TtsEngineInfo>`
  - `POST /api/tts/speak` ← `TtsRequest { text, engine?, language?, pitch?, rate?, volume? }`
  - `POST /api/tts/stop`
  - `GET/PUT /api/tts/settings`、`POST /api/tts/default-engine`、`POST /api/tts/test`、`GET/POST /api/tts/engine-params`、`GET/POST /api/tts/replace-rules` 等
- AI 对话：`GET /ws/ai-chat`（WebSocket 流式）
- 投屏：`GET /ws/mirror` + `GET /ws/mirror/audio`（WebSocket）

### 5.2 TTS 广播协议（APK 端 `TtsReceiver` 监听）
- `com.taskmod.app.TTS_SPEAK`：extras = `text`(String 必填) / `engine`(String 可选) / `language`(String 可选) / `rate`(Float 可选) / `pitch`(Float 可选) / `volume`(Float 可选)
- `com.taskmod.app.TTS_STOP`
- `com.taskmod.app.TTS_INIT`
- Rust 服务端 `exec_speak_via_apk()` 通过 `am broadcast -a com.taskmod.app.TTS_SPEAK --es text xxx --ef rate 1.0` 调用。

### 5.3 TtsManager 单例（`android/.../TtsManager.kt`）
- `fun init(context: Context, preferredEngine: String? = null)`
- `fun speak(text: String, language: String = "system", rate: Float = 1.0f, pitch: Float = 1.0f): Boolean`
- `fun setEngineAndSpeak(text: String, engine: String?, language: String?, rate: Float, pitch: Float): Boolean`
- `fun stop()` / `fun shutdown()` / `fun reinit(context: Context, preferredEngine: String? = null)`
- StateFlow: `isAvailable` / `isPlaying` / `langMissingData` / `lastInitStatus` / `lastSpeakResult` / `lastLanguageResult`
- `fun getDiagnosticInfo(): TtsDiagnosticInfo` / `fun getEngines(): List<String>` / `fun isInitialized(): Boolean`

### 5.4 配置路径常量（`server/src/config.rs`）
- `TASKMOD_DIR` = `/sdcard/TaskMod/`（或 `/data/adb/TaskMod/` for Magisk 持久化）
- `SCRIPTS_DIR` / `SCREENSHOTS_DIR` / `WORKFLOWS_DIR` 等
- 端口：`get_listen_port()` 读 `app_settings.json`，默认 9527

## 6. 下一步任务（按优先级，逐项勾选）

- [x] **验证 v1.0.15 Build & Release 全绿**：Build & Release (run 32014268222) success，Release v1.0.15 已产出 TaskMod-1.0.15.zip (5.38 MB) + TaskMod-app-1.0.15.apk (7.09 MB)。CI (run 32014263189) success。build-apk.yml 修复后 workflow_dispatch run 32015753734 success。
- [ ] **验证 v1.0.14 TTS 原生调用**：CI 已全绿（run 32013772236），待设备实测 TTS 是否成功（Rust 服务端优先调 APK 广播，APK 内 TtsManager 直接用 Android TextToSpeech API）。
- [ ] （按用户后续需求补充）

## 7. 编码约定（强制）

### 7.1 Rust（server/）
- **edition 2021**，`cargo fmt` 格式，`cargo clippy -- -D warnings` 零警告。
- 错误处理：用 `anyhow::Result` 顶层 + `?` 传播；API handler 返回 `ApiResponse::err(msg)` 而非 panic。
- 异步：`tokio` runtime，handler 用 `async fn`，阻塞操作用 `tokio::task::spawn_blocking`。
- 日志：`tracing::info!` / `warn!` / `error!`，不用 `println!`。
- 命令执行：`tokio::process::Command`（异步），不用 `std::process::Command`（阻塞）。
- 序列化：`serde` derive，`serde_json`，API 请求/响应用 `#[derive(Deserialize)]` / `#[derive(Serialize)]`。
- 热加载服务（MQTT/邮件/TTS）：按需加载，未配置时零内存占用，用 `lazy_static` 或 `tokio::spawn` 按需启动。

### 7.2 Kotlin（android/）
- **包名 `com.taskmod.app`**，禁止引入 `com.lxseek.chat`（Agora 包名）。
- `@Volatile` 用于跨线程可见性字段；`StateFlow` 用于可观察状态。
- 主线程操作：`Handler(Looper.getMainLooper()).post { }`。
- 日志：`android.util.Log.d/e/w/i`，tag 用 companion `TAG` 常量。
- BroadcastReceiver：`onReceive` 包 `try-catch(Throwable)` 防崩溃。
- Android 资源：`R.string.*` / `R.drawable.*`，新增字符串需 `values/strings.xml`（默认英文）+ `values-zh/strings.xml`（中文）同步。

### 7.3 Dioxus/eq_ui（frontend/）
- **Dioxus 0.7** + **eq_ui 0.5**，遵循 `CLAUDE.md` 的 DeepSeek++ 设计语言。
- 深色/浅色双模式必须完整适配。
- 无 Emoji 堆砌，每个图标对应明确功能含义。
- 组件用 `eq_ui` 原子组件（EqCard/EqButton 等），保持一致性。

### 7.4 通用
- **不主动 commit**，除非用户明确要求。
- **不写未经请求的文档/README**。
- **不加冗余注释**（代码自解释优先，复杂逻辑才注释）。
- **借鉴其他项目代码时**：适配本项目的包名/依赖/命名，移除源项目特有依赖（如 Agora 的 SherpaTtsEngine/AppLog），保留核心能力。

## 8. 常用命令

### 8.1 本地静态检查（push 前自检）
```bash
# Rust 服务端（需要有 Rust 工具链）
cd server
cargo check                  # 类型/借用检查
cargo clippy -- -D warnings  # lint（CI 会执行）
cargo fmt --check            # 格式检查（CI 会执行）

# 前端（需要有 Dioxus CLI）
cd frontend
dx check                     # Dioxus 0.7 类型检查

# Kotlin（本地无法编译，人工查 import/类型）
```

### 8.2 Git 操作
```bash
git status                   # 检查残留修改
git add -A && git status     # 确认所有修改已 staged
git commit -m "feat: 描述"   # commit（用户明确要求时才执行）
git push origin main         # 触发 ci.yml
git push origin v1.0.15      # 触发 build.yml 发版
```

### 8.3 CI 监控
```bash
gh run watch                 # 监控最新 run
gh run view --log-failed     # 查看失败日志
gh run list --limit 5        # 列出最近 5 个 run
```

### 8.4 设备测试（装好 APK + Magisk 模块后）
```bash
# 访问 Web 管理面板
curl http://设备IP:9527/api/status

# 测试 TTS（优先调 APK 原生 TtsManager）
curl -X POST http://设备IP:9527/api/tts/speak \
  -H "Content-Type: application/json" \
  -d '{"text":"你好，这是 TTS 测试","language":"zh","rate":1.0}'

# 查看 TTS 引擎列表
curl http://设备IP:9527/api/tts/engines

# 停止 TTS
curl -X POST http://设备IP:9527/api/tts/stop
```

## 9. 变更日志（追加新行，最新在上）

- **2026-08-17**：修复 build-apk.yml 的 "Build Rust server for Android (arm64)" 步骤。原用手动 `~/.cargo/config.toml` + `cargo build`，ring 的 cc-rs 找不到 `aarch64-linux-android-clang`（不带 API level）→ 加 CC/CXX/AR env（commit `a926a6c`）后 openssl-sys vendored 又因缺 RANLIB 等失败 → 最终改用 `cargo-ndk`（与 build.yml build-server job 一致，commit `c506ab7`），workflow_dispatch 验证 run 32015753734 **success**。现 build-apk.yml 与 build.yml 走同一 cargo-ndk 路径，CI 全绿。
- **2026-08-17**：发版 v1.0.15。bump 三处版本号（`server/Cargo.toml` version 1.0.14→1.0.15；`module.prop` version 1.0.14→1.0.15 / versionCode 1000014→1000015；`android/app/build.gradle` versionCode 1→1000015 / versionName "1.0.0"→"1.0.15"）。commit `eb5dd70` "chore: bump version to v1.0.15"，tag `v1.0.15` 已 push。触发 Build & Release (run 32014268222) + Build APK (run 32014268238) + CI (run 32014263189)，均 in_progress。待全绿后产出 Release。下一步：监控三 run 至 success，确认 Release 产物。
- **2026-08-17**：借鉴 Agora 的 TTS 调用方式到 TaskMod。新建 `TtsManager.kt`（333 行，多引擎切换+看门狗+诊断日志+init 重试+stale 回调防护+stripMarkdown+setLanguage 三级回退+主线程 speak）+ `TtsReceiver.kt`（54 行，BroadcastReceiver）；修改 `AndroidManifest.xml`（注册 TtsReceiver + `<queries>` TTS_SERVICE）+ `TaskModApp.kt`（onCreate 调 TtsManager.init）+ `server/src/api/tts.rs`（新增 `exec_speak_via_apk`，`exec_speak` 优先广播调 APK，fallback shell）。待 CI 验证。下一步：push 确认 CI 全绿，设备测试 TTS。
- **2026-08-17**：创建 `AGENTS.md` 代理工作指引，借鉴 Agora 项目结构，适配 TaskMod 多语言技术栈（Rust + Dioxus + Kotlin + Magisk）。

## 10. 参考索引

### 10.1 关键文件快速跳转
| 要改什么 | 看哪个文件 |
|---|---|
| API 路由注册 | `server/src/main.rs`（第 343 行起 `Router::new()`） |
| TTS 调用逻辑 | `server/src/api/tts.rs`（`exec_speak_via_apk` + `exec_speak`） |
| TTS 配置模型 | `server/src/data/tts_config.rs` |
| AI 对话核心 | `server/src/api/ai.rs`（`build_api_url` + 流式 SSE） |
| AI Hub | `server/src/api/ai_hub.rs` |
| 投屏控制 | `server/src/api/mirror.rs` |
| APK TTS 原生 | `android/.../TtsManager.kt` + `TtsReceiver.kt` |
| APK 入口 | `android/.../TaskModApp.kt`（Application） |
| APK 清单 | `android/app/src/main/AndroidManifest.xml` |
| APK 构建 | `android/app/build.gradle` |
| 服务端依赖 | `server/Cargo.toml` |
| 前端依赖 | `frontend/Cargo.toml` |
| CI 配置 | `.github/workflows/{ci,build,build-apk}.yml` |
| Magisk 模块属性 | `module.prop` |
| UI 设计规范 | `CLAUDE.md` |

### 10.2 借鉴源项目
- **Agora**（`D:\GitHub\Agora`）：Android Kotlin 原生 LLM 客户端。借鉴了其 `TtsManager.kt`（多引擎切换+看门狗+诊断日志+init 重试+stale 回调防护）到 TaskMod APK 端。Agora 的 `AGENTS.md` 结构是本文件的模板来源。