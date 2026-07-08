# TaskMod

![GitHub](https://img.shields.io/github/license/ojbkxc/TaskMod)
![GitHub stars](https://img.shields.io/github/stars/ojbkxc/TaskMod)

一个基于 Rust 的 Android 设备自动化管理工具，支持屏幕镜像、AI 控制、脚本执行、MQTT、邮件通知等功能。

## 项目地址

https://github.com/ojbkxc/TaskMod

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

## 目录结构

```
TaskMod/
├── .github/workflows/     # CI/CD 配置
├── META-INF/              # Magisk 模块配置
├── sdcard/TaskMod/        # 用户数据目录
│   ├── scripts/           # 脚本目录
│   ├── screenshots/       # 截图目录
│   ├── workflows/         # 工作流目录
│   ├── schedule.conf      # 定时任务配置
│   ├── email.conf         # 邮件配置
│   ├── ai.conf            # AI 供应商配置
│   └── mqtt.conf          # MQTT配置（可选）
├── server/                # Rust 服务器源码
│   ├── src/               # 源代码
│   │   ├── api/           # API 路由
│   │   ├── data/          # 数据模型
│   │   ├── utils/         # 工具模块
│   │   │   ├── email.rs   # 邮件功能（统一实现）
│   │   │   ├── mqtt.rs    # MQTT功能（纯Rust实现）
│   │   │   ├── adb.rs     # ADB命令封装
│   │   │   └── service_manager.rs  # 服务热加载管理
│   │   └── main.rs        # 入口文件
│   ├── static/            # Web 静态资源
│   └── Cargo.toml         # 依赖配置
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

1. 下载最新 release 包
2. 在 Magisk Manager 中刷入模块
3. 重启设备
4. 访问 http://设备IP:9527

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

启动后访问: http://设备IP:9527

功能模块:
- 仪表盘 - 系统状态概览
- 任务管理 - 定时任务管理
- 日志查看 - 实时日志
- 截图管理 - 截图拍摄与管理
- 脚本管理 - 脚本编辑与执行
- 配置编辑 - 配置文件管理
- 命令执行 - 快捷命令执行
- 邮件通知 - 邮件配置与发送
- 语音播报 - TTS文本转语音
- 投屏控制 - 屏幕镜像与控制
- AI 助手 - AI 对话与控制
- 服务设置 - MQTT、邮件、TTS服务配置

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
```

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
| end | 结束节点 | 无 |

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

## 许可证

MIT License
