# TaskMod

![GitHub](https://img.shields.io/github/license/ojbkxc/TaskMod)
![GitHub stars](https://img.shields.io/github/stars/ojbkxc/TaskMod)

一个基于 Rust 的 Android 设备自动化管理工具，支持屏幕镜像、AI 控制、脚本执行、MQTT、邮件通知等功能。

## 项目地址

https://github.com/ojbkxc/TaskMod

## UI 设计

采用现代化侧边栏导航设计，参照 deepseek-pp-main 设计风格：

- **侧边栏导航** - 仪表盘 | AI助手 | 投屏 | 知识库 | 任务 | 脚本 | TTS | 配置 | 日志
- **深色/浅色主题** - 支持一键切换
- **GitHub 链接** - 侧边栏底部快速访问项目仓库
- **响应式布局** - 适配不同屏幕尺寸

## 功能特性

### 🏠 基础功能
- **定时任务管理** - 支持每天/指定星期/间隔执行脚本
- **脚本管理** - 增删改查 scripts 目录下的脚本文件
- **截图管理** - 拍摄、预览、删除截图
- **日志查看** - 实时查看执行日志
- **命令执行** - 执行任意 shell 命令
- **配置管理** - 在线编辑配置文件

### 📧 邮件通知（热加载）
- **脚本执行通知** - 脚本执行完成自动发送邮件
- **附件支持** - 可添加截图等文件作为邮件附件
- **重试机制** - 网络错误时自动重试
- **变量模板** - 支持 {script} {time} {date} {result} 变量
- **热加载** - 未配置则零内存占用

### 📱 屏幕镜像
- **实时屏幕镜像** - H.264 编码，低延迟传输
- **触摸控制** - 鼠标/触摸点击、滑动操作
- **键盘输入** - 支持文本输入和按键模拟
- **息屏操作** - 息屏状态下继续操作，不影响后续亮度
- **麦克风音频** - 实时音频传输
- **浏览器录屏** - 边录制边下载，无需存储在服务器

### 🔊 TTS 语音播报（热加载）
- **文本转语音** - 在手机上播放文本语音
- **多引擎选择** - 支持系统多个TTS引擎切换
- **语速音调调节** - 可调整语速、音调、音量
- **热加载** - 按需调用，不占用常驻内存

### 🤖 AI 助手
- **多供应商配置** - 支持 OpenAI 兼容接口（Moonshot、DeepSeek、千问等）
- **多会话窗口** - 支持同时开启多个对话会话
- **图片生成** - AI生成的图片自动显示
- **自然语言对话** - 与 AI 实时对话
- **ADB 操作手机** - AI 自动调用 ADB 命令操作手机
  - 点击、滑动、按键、输入文本
  - 启动/停止应用、获取 WiFi 信息
  - 获取设备信息、电池信息、运行应用列表
  - 重启、关机（需 root）、清除应用数据（需 root）
- **TTS 语音播报** - AI 调用系统 TTS 发出声音（支持小爱同学）
- **脚本管理** - AI 创建、编辑、删除、执行脚本
- **日志查看** - AI 查看运行日志
- **设备信息感知** - AI 自动获取屏幕分辨率和脚本列表

### 🧠 AI Hub（高级AI功能）
- **知识库页面** - 独立的侧边栏标签，包含记忆、预设、技能、保存项管理
- **对话历史** - 自动保存对话记录，支持恢复、归档、置顶、导出（Markdown/JSON）
- **Prompt 预设** - 保存常用 system prompt 模板，一键切换，支持活跃预设选择
- **Prompt 控制面板** - 记忆注入开关、系统提示词开关、预设注入频率（首次/每轮/关闭）、强制回复语言
- **记忆系统** - 跨对话持久化记忆，支持分类、标签、搜索、置顶，4种类型（用户偏好/反馈/主题/参考），2种作用域（全局/项目），智能关键词匹配注入，访问计数和token预算限制
- **Skill 系统** - 热加载 Skill 文件，定义可复用的 prompt 模板和变量，支持分类和来源标记，AI对话时自动注入已启用的Skill
- **场景模板** - 内置5个场景（总结/解释/翻译中英/调试分析），支持自定义，{text}变量替换
- **保存项** - 保存常用代码片段、命令、书签，支持来源URL、搜索和一键复制
- **项目上下文** - 按项目分组指令和上下文，可自动注入到 AI 对话，支持关联对话和记忆
- **MCP 协议** - 集成 Model Context Protocol，支持 stdio/SSE/HTTP 传输，热加载配置
- **截图+AI 视觉分析** - 截取设备屏幕，发送给 AI 进行视觉分析和操作建议
- **对话导出** - 将对话导出为 Markdown 或 JSON 格式
- **多供应商回退** - Provider 按顺序尝试直到成功，保证高可用
- **记忆智能注入** - 根据用户消息关键词自动匹配最相关的记忆，按标签/名称/内容评分排序，支持归档过期记忆
- **定时任务AI控制** - AI 可查看、添加、删除、修改定时任务

### 🔄 工作流引擎
- **可视化工作流** - 通过节点连接构建自动化流程
- **丰富节点类型** - start、script、command、delay、email、email_attachment、tts、end
- **分支执行** - 通过连线控制执行路径
- **附件支持** - 邮件节点可配置截图附件

### 📡 MQTT支持（热加载）
- **设备状态发布** - 电量、温度、运行状态实时上报到MQTT broker
- **远程命令执行** - 订阅topic接收ADB命令，实现外部系统控制
- **智能家居集成** - 对接Home Assistant等平台
- **热加载** - 未配置则零内存占用，仅在启用时加载

### 🔧 服务管理（热加载）
- **服务状态监控** - 实时查看各服务状态
- **热加载/热卸载** - 服务按需加载，用完自动释放
- **引用计数** - 智能管理服务生命周期
- **自动清理** - 空闲服务自动卸载释放内存

## 项目结构

```
TaskMod/
├── .github/workflows/     # CI/CD 配置
│   ├── build.yml          # 主构建（服务端 + Magisk模块 + APK）
│   ├── build-apk.yml      # APK 独立构建
│   └── ci.yml             # 代码检查
├── META-INF/              # Magisk 模块配置
├── sdcard/TaskMod/        # 用户数据目录（APK 与模块共享）
│   ├── app_settings.json  # APK 统一配置（端口、IP、域名、自启动）
│   ├── scripts/           # 脚本目录
│   ├── screenshots/       # 截图目录
│   ├── workflows/         # 工作流目录
│   ├── chat_history/      # AI对话历史
│   ├── memory/            # AI记忆系统
│   ├── skills/            # AI Skill文件（热加载）
│   ├── saved_items/       # AI保存项
│   ├── projects/          # AI项目上下文
│   ├── mcp/               # MCP服务器配置（热加载）
│   ├── presets.json       # Prompt预设
│   ├── prompt_settings.json # Prompt注入设置
│   ├── scenarios.json     # 场景模板
│   ├── schedule.conf      # 定时任务配置
│   ├── email.conf         # 邮件配置
│   ├── ai.conf            # AI 供应商配置
│   └── mqtt.conf          # MQTT配置（可选）
├── server/                # Rust 服务器源码
│   ├── src/               # 源代码
│   │   ├── api/           # API 路由
│   │   │   ├── ai.rs      # AI对话核心
│   │   │   ├── ai_hub.rs  # AI Hub
│   │   │   ├── mirror.rs  # 投屏控制
│   │   │   ├── system.rs  # 系统管理
│   │   │   ├── tasks.rs   # 定时任务
│   │   │   ├── scripts.rs # 脚本管理
│   │   │   └── tts.rs     # 语音播报
│   │   ├── tools/         # AI Tool Calling
│   │   ├── utils/         # 工具模块
│   │   ├── config.rs      # 路径常量
│   │   └── main.rs        # 入口
│   ├── static/            # Web 静态资源
│   │   ├── index.html     # HTML 结构
│   │   ├── style.css      # 样式
│   │   └── app.js         # JavaScript 逻辑
│   └── Cargo.toml
├── android/               # Android APK 源码
│   ├── app/src/main/java/com/taskmod/app/
│   │   ├── ConfigManager.kt    # 统一配置（/sdcard/TaskMod/app_settings.json）
│   │   ├── NetworkHelper.kt    # 多网卡 IP 检测
│   │   ├── ServerManager.kt    # 服务进程管理
│   │   ├── MainActivity.kt     # 主界面
│   │   ├── WebViewActivity.kt  # 内嵌浏览器
│   │   ├── SettingsActivity.kt # 设置页
│   │   ├── MagiskGuideActivity.kt # 模块安装引导
│   │   ├── TaskModService.kt   # 前台服务
│   │   ├── TaskModApp.kt       # Application
│   │   ├── RootHelper.kt       # Root 命令执行
│   │   ├── UpdateChecker.kt    # 自动更新
│   │   ├── BootReceiver.kt     # 开机启动
│   │   ├── widget/             # 桌面小组件
│   │   └── tiles/              # Quick Settings 磁贴
│   └── build.gradle
├── customize.sh           # Magisk 安装脚本
├── service.sh             # 服务启动脚本
└── module.prop            # Magisk 模块属性
```

## 编译方式

### 注意
本项目**只在 GitHub Actions 上编译**，无需本地编译环境。

推送代码到 GitHub 后，CI/CD 会自动编译并生成 release 包。

### 编译目标
- 架构: `aarch64-linux-android`
- 平台: Android (Magisk 模块)

## 安装方法

### 方式一：Magisk 模块（推荐，完整功能）

1. 从 [Releases](https://github.com/ojbkxc/TaskMod/releases) 下载最新 zip 文件
   - 文件名格式：`TaskMod-版本号.zip`
   - 下载地址示例：`https://github.com/ojbkxc/TaskMod/releases/download/v1.0.4/TaskMod-1.0.4.zip`
2. 打开 Magisk App → 模块 → 从本地安装
3. 选择下载的 zip 文件
4. 等待安装完成后重启设备
5. 浏览器访问 `http://设备IP:9527`

**模块优势：**
- 内核级后台保活，几乎不会被杀
- 开机自动启动
- 完整的 ADB 命令执行权限

### 方式二：APK 安装（无需 Recovery）

1. 从 [Releases](https://github.com/ojbkxc/TaskMod/releases) 下载最新 APK 文件
   - 文件名格式：`TaskMod-v版本号-debug.apk`
2. 允许安装未知来源应用
3. 安装并打开 APK
4. 首次启动会自动检测环境：
   - 有 Magisk → 引导下载并刷入模块（推荐）
   - 无 Magisk → 使用内置服务（功能受限）

**APK 功能：**
- 通知栏常驻服务（保活）
- 快捷操作（截屏/解锁/重启）
- Quick Settings 磁贴（下拉快捷面板）
- 桌面小组件
- 内置 WebView 管理面板
- 自动更新检测
- Root 检测与 Magisk 模块引导
- 统一配置管理（与 Magisk 模块共享 `/sdcard/TaskMod/`）
- 多网卡 IP 自动检测（WiFi/以太网/蜂窝/VPN）
- 自定义端口（默认 9527，可修改）
- 自定义 IP 和域名支持（DDNS 等场景）

**APK 设置页：**
- 开机自启动开关
- 服务端口配置（1024-65535）
- 自定义 IP 地址
- 自定义域名/完整 URL（如 `http://myphone.ddns.net`）
- 实时显示所有可用访问地址

**APK 注意事项：**
- 设备控制功能（ADB 命令、截屏、触控）需要 Root 权限
- AI 聊天功能无需 Root
- 配置文件存储在 `/sdcard/TaskMod/app_settings.json`，与 Magisk 模块共享
- 建议同时安装 Magisk 模块获得完整功能

### 方式三：APK + Magisk 模块组合（最佳体验）

1. 先安装 APK（获取 UI 和快捷操作）
2. 在 APK 内点击"安装模块"按钮
3. APK 会自动下载最新 Magisk 模块
4. 按提示在 Magisk 中刷入模块
5. 重启设备

这样既有 APK 的便捷操作（通知栏、磁贴、小组件），又有模块的稳定保活。

## 使用说明

### 定时任务配置 (schedule.conf)

```
# 每天 07:30 执行
07:30 midea.sh

# 周一到周五 08:00 执行
08:00 1,2,3,4,5 work.sh

# 每5分钟执行一次
every 5 check.sh
```

### Web 管理面板

启动后访问: http://设备IP:9527（端口可在 APK 设置中修改）

**侧边栏导航:**
- 仪表盘 - 系统状态概览与快捷操作
- AI助手 - AI对话与设备控制（支持工具调用、流式响应）
- 投屏 - 实时屏幕镜像与触摸控制
- 知识库 - 记忆、预设、技能、保存项管理
- 任务 - 定时任务管理
- 脚本 - 脚本编辑与执行
- TTS - 语音播报配置
- 配置 - 邮件、MQTT、系统命令配置
- 日志 - 实时日志查看
- 主题 - 深色/浅色主题切换
- GitHub - 快速访问项目仓库

### AI 使用示例

```
帮我返回主屏幕        → AI 执行 input keyevent 3
点击屏幕中心          → AI 执行 input tap 480 640
打开设置应用          → AI 执行 am start com.android.settings
讲个笑话              → AI 直接回答
列出所有脚本          → AI 返回脚本列表
执行 midea.sh 脚本    → AI 运行脚本
创建脚本 test.sh      → AI 创建脚本
查看最近 50 行日志    → AI 返回日志
帮我说'你好'          → AI 调用 TTS 播放语音
生成一张猫的图片      → AI 返回图片URL并显示
帮我建个每天8点的定时任务 → AI 调用 add_task 创建任务
把任务2改成9点执行    → AI 调用 modify_task 修改任务
看看有哪些定时任务    → AI 调用 list_tasks 列出任务
截图分析下当前界面    → AI 截图并用视觉模型分析
```

### AI 工具列表

| 类别 | 工具名 | 说明 |
|------|--------|------|
| ADB | adb_tap | 点击屏幕坐标 |
| ADB | adb_swipe | 滑动屏幕 |
| ADB | adb_keyevent | 模拟按键 |
| ADB | adb_input_text | 输入文本 |
| ADB | adb_screencap | 截图 |
| ADB | adb_command | 执行shell命令 |
| ADB | adb_start_app | 启动应用 |
| ADB | adb_stop_app | 停止应用 |
| ADB | adb_clear_app_data | 清除应用数据 |
| ADB | adb_tts | 语音播报 |
| ADB | adb_reboot | 重启设备 |
| ADB | adb_shutdown | 关机 |
| ADB | get_device_info | 获取设备信息 |
| ADB | get_battery_info | 获取电池信息 |
| ADB | get_wifi_info | 获取WiFi信息 |
| ADB | get_running_apps | 获取运行应用 |
| 脚本 | list_scripts | 列出所有脚本 |
| 脚本 | read_script | 读取脚本内容 |
| 脚本 | write_script | 创建/编辑脚本 |
| 脚本 | delete_script | 删除脚本 |
| 脚本 | run_script | 执行脚本 |
| 脚本 | view_logs | 查看日志 |
| 任务 | list_tasks | 查看定时任务 |
| 任务 | add_task | 添加定时任务 |
| 任务 | delete_task | 删除定时任务 |
| 任务 | modify_task | 修改定时任务 |
| 任务 | list_available_scripts | 列出可用脚本 |

### 工作流节点

| 节点类型 | 说明 | 配置项 |
|---------|------|--------|
| start | 开始节点 | 无 |
| script | 执行脚本 | 脚本文件名 |
| command | 执行命令 | 命令内容 |
| delay | 延时等待 | 秒数 |
| email | 发送邮件 | 收件人、主题、内容 |
| email_attachment | 带附件邮件 | 收件人、主题、内容、附件列表 |
| tts | 语音播报 | 文本内容、TTS引擎 |
| ai_generate | AI生成内容 | provider_id、prompt、output_var |
| condition | 条件分支 | expression、true_next、false_next |
| mqtt_publish | MQTT发布消息 | topic、payload |
| end | 结束节点 | 无 |

## APK 配置文件

APK 与 Magisk 模块共享同一份配置，存储在 `/sdcard/TaskMod/app_settings.json`：

```json
{
  "port": 9527,
  "customUrl": "",
  "customIp": "",
  "autoStart": true
}
```

| 字段 | 说明 | 默认值 |
|------|------|--------|
| port | 服务端口 | 9527 |
| customUrl | 自定义域名/完整URL（优先级最高） | 空 |
| customIp | 自定义IP地址（配合端口使用） | 空 |
| autoStart | 开机自动启动 | true |

**地址解析优先级：**
1. `customUrl` → 直接使用（如 `http://myphone.ddns.net`），端口可选
2. `customIp` + `port` → `http://{ip}:{port}`
3. 自动检测 → `http://{WiFi/以太网IP}:{port}`

## 常用 ADB 命令

```bash
唤醒屏幕:       input keyevent 26
锁屏:           input keyevent 224
返回:           input keyevent 4
Home:           input keyevent 3
点击屏幕:       input tap X Y
滑动屏幕:       input swipe X1 Y1 X2 Y2
输入文本:       input text "内容"
启动应用:       am start -n 包名/Activity
强制停止:       am force-stop 包名
截图:           screencap -p /path/to/save.png
录屏:           screenrecord /path/to/save.mp4
```

## 配置文件

### email.conf

```
enable_notify=true
smtp_server=smtp.qq.com
smtp_port=587
username=your_email@qq.com
password=your_auth_code
from=your_email@qq.com
to=recipient@example.com
subject=TaskMod 通知 - {script}
body=脚本 {script} 已于 {date} {time} 执行完成\n\n{result}
timeout_secs=30
max_retries=3
retry_interval=1
```

### 模板变量

- `{script}` - 脚本名称
- `{time}` - 执行时间 (HH:MM:SS)
- `{date}` - 执行日期 (YYYY-MM-DD)
- `{result}` - 脚本输出结果

## MQTT配置

MQTT是轻量级物联网消息协议，可用于：
- 发布设备状态（电量、温度、运行状态）到MQTT broker
- 订阅topic接收远程命令，实现外部系统触发ADB操作
- 集成Home Assistant等智能家居平台

### 配置文件 (mqtt.conf)

```
# enabled=true 启用MQTT功能
# 不配置或enabled=false则不加载MQTT，零内存占用（热加载）
enabled=false
broker=tcp://localhost:1883
topic_prefix=taskmod
username=
password=
client_id=taskmod-device
```

### 配置说明

| 配置项 | 说明 | 默认值 |
|--------|------|--------|
| enabled | 是否启用MQTT | false |
| broker | MQTT broker地址 | tcp://localhost:1883 |
| topic_prefix | 主题前缀 | taskmod |
| username | 用户名（可选） | 空 |
| password | 密码（可选） | 空 |
| client_id | 客户端ID | taskmod-device |

### 工作机制

- **热加载**：仅在配置文件存在且 `enabled=true` 时才启动MQTT连接
- **不配置则零内存占用**：未启用时不占用任何系统资源
- **自动重连**：连接断开后自动尝试重连
- **心跳保活**：30秒心跳间隔

### MQTT Topic

| Topic | 方向 | 说明 |
|-------|------|------|
| `{prefix}/status` | 发布 | 设备状态JSON（每分钟更新） |
| `{prefix}/cmd` | 订阅 | 接收远程命令 |
| `{prefix}/result` | 发布 | 命令执行结果 |

### 使用示例

**发布设备状态**：
```json
{
  "device_model": "Xiaomi 13",
  "android_version": "14",
  "battery_capacity": "85",
  "battery_temperature": "32.5",
  "battery_status": "Charging",
  "uptime": "up 2 days, 10:30",
  "screen_size": "Physical size: 1080x2400"
}
```

**远程执行命令**：
```bash
# 通过MQTT发送命令（将{prefix}替换为你的topic_prefix）
mosquitto_pub -t "taskmod/cmd" -m "input keyevent 3"

# 接收执行结果
mosquitto_sub -t "taskmod/result"
```

## 热加载机制

TaskMod 采用智能热加载机制，优化内存占用和启动速度：

### 核心原则
- **按需加载**：服务仅在使用时加载
- **自动卸载**：空闲一段时间后自动释放内存
- **引用计数**：多个功能共享同一服务时不卸载
- **零资源占用**：未启用的服务不占用任何内存

### 热加载服务列表
| 服务 | 加载时机 | 卸载条件 |
|------|---------|---------|
| MQTT | 配置启用后自动加载 | 配置禁用或连接失败 |
| 邮件 | 发送邮件时自动加载 | 发送完成后自动释放 |
| TTS | 调用语音时自动加载 | 播放完成后自动释放 |
| AI | 打开AI助手页面时加载 | 关闭页面后释放 |

## API 接口

### TTS 接口
```
GET  /api/tts/engines    - 获取TTS引擎列表
POST /api/tts/speak      - 发送语音播放命令
```

### 邮件接口
```
GET  /api/email/config   - 获取邮件配置
PUT  /api/email/config   - 更新邮件配置
POST /api/send-email     - 发送邮件（支持附件）
```

### 服务管理接口
```
GET  /api/service/status - 获取所有服务状态
POST /api/service/load   - 手动加载服务
POST /api/service/unload - 手动卸载服务
```

## 注意事项

- 修改配置后无需重启，30秒内自动生效
- 以 `#` 开头的行为注释
- 脚本文件需要有执行权限
- Web 服务默认监听 `0.0.0.0:9527`
- 邮件通知需要先配置并启用才会生效
- AI 功能需要联网才能调用外部 API
- AI 操作手机需要设备已获取 ROOT 权限或已开启 ADB 调试
- TTS 语音功能需要系统支持（支持小爱同学等语音助手）
- MQTT 使用纯Rust实现（rumqttc），无C依赖，避免交叉编译问题
- 邮件功能使用统一的utils/email.rs实现，无重复代码

## 更新日志

### v1.0.4 (2026-07-10)

**Bug 修复：**
- **定时任务 interval 字段丢失** - `add_task` API 未写入 `interval` 字段，导致间隔任务配置丢失
- **schedule.conf 格式不一致** - Rust 后端使用 `|` 分隔符，`service.sh` 使用空格分隔，现已统一支持两种格式（优先管道分隔，兼容旧格式）
- **使用量统计日期计算错误** - `chrono_date()` 使用简单的天数除法计算日期（不考虑闰年），修正为使用 `chrono::Local` 获取准确日期
- **ADB 文本输入转义顺序错误** - 单引号/双引号转义应在反斜杠转义之后执行，导致双重转义问题，影响投屏和 AI 工具的文本输入
- **AI截图分析API路径错误** - `call_ai_image_analyze` 缺少 `/v1` 前缀，导致截图+AI视觉分析功能无法正常工作
- **文件上传命令注入风险** - `upload_file_to_device` 未校验文件名，添加路径穿越和特殊字符过滤
- **TTS播放无声音** - `cmd tts speak` 只尝试单一路径且同步阻塞，改为多路径容错 + 异步后台播放 + 详细错误日志
- **PCM音频录制无限重试** - `tinycap` 在设备无麦克风时无限刷屏，改为3次失败后停止并记录详细错误
- **Build版本号不同步** - `build.yml` 的 APK 构建未同步版本号，现已统一从 tag/workflow_dispatch 获取

**功能补充：**
- **工作流节点类型扩展** - 新增 `ai_generate`（AI生成）、`condition`（条件分支）、`mqtt_publish`（MQTT发布）节点类型
- **APK现代化重构** - 参照 ClashMetaForAndroid，全部 `Thread` 替换为 Kotlin 协程，Service 实现 `CoroutineScope` 自动管理生命周期
- **APK WebView缓存优化** - 本地服务器使用 `LOAD_NO_CACHE` 策略，确保每次加载最新UI
- **APK版本号显示** - WebView User-Agent 包含版本号便于调试
- **投屏全屏支持** - 新增全屏按钮，全屏时隐藏侧边栏，纯黑背景沉浸式显示
- **投屏声音同步** - 通过 Web Audio API + AudioWorklet 实时播放设备音频（48kHz 16-bit mono PCM）
- **投屏鼠标滚轮** - 鼠标滚轮上下滚动映射为设备上下滑动手势
- **任务/脚本页面重构** - 统一卡片式布局，任务支持编辑/类型选择/cron预设，脚本支持新建/代码编辑器(Tab缩进/Ctrl+S保存/行号显示)
- **设备控制页面整合** - 主菜单"投屏"改为"设备"，截图/解锁/重启移入设备页面，新增ADB命令输入框+常用命令快捷按钮

## 许可证

MIT License
