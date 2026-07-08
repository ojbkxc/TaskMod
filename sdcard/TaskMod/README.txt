==============================
        TaskMod 使用说明
==============================

【目录结构】
  /sdcard/TaskMod/
  ├── schedule.conf        -- 定时任务配置
  ├── email.conf           -- 邮件配置（自动生成）
  ├── scripts/             -- 脚本目录
  ├── screenshots/         -- 截图目录
  └── README.txt           -- 本说明文件

【schedule.conf 格式】

  1) 指定时间 + 指定星期:
     HH:MM 星期 脚本名
     示例: 07:30 1,2,3,4,5 midea.sh
     说明: 周一到周五 07:30 执行 midea.sh
     星期: 1=周一 2=周二 3=周三 4=周四 5=周五 6=周六 7=周日

  2) 指定时间, 每天执行:
     HH:MM 脚本名
     示例: 08:10 midea.sh
     说明: 每天 08:10 执行 midea.sh

  3) 每隔N分钟执行一次:
     every 分钟数 脚本名
     示例: every 5 check.sh
     说明: 每5分钟执行一次 check.sh

【Web 管理面板】

  启动后访问: http://设备IP:9527

  功能模块:
  ┌─────────────────────────────────────────────────────────┐
  │ 仪表盘 │ 任务管理 │ 日志查看 │ 截图管理 │ 脚本管理 │
  │ 配置编辑 │ 命令执行 │ 邮件通知 │
  └─────────────────────────────────────────────────────────┘

  1. 仪表盘
     - 显示系统运行时间、任务数量、截图数量、磁盘信息
     - 快捷操作按钮

  2. 任务管理
     - 查看所有定时任务
     - 添加任务（支持每天/指定星期/间隔执行）
     - 删除任务

  3. 日志查看
     - 实时查看执行日志（自动刷新）
     - 清空日志

  4. 截图管理
     - 拍摄新截图
     - 预览截图（点击放大）
     - 删除截图

  5. 脚本管理
     - 新建脚本
     - 编辑脚本
     - 执行脚本

  6. 配置编辑
     - 在线编辑 schedule.conf
     - 修改后自动生效（30秒内）

  7. 命令执行
     - 输入任意 shell 命令执行
     - 常用命令快捷按钮（唤醒屏幕、Home、返回、截图等）

  8. 邮件通知
     - 启用/禁用脚本执行完成自动通知
     - SMTP 服务器配置
     - 收件人配置（支持多个，逗号分隔）
     - 主题/内容模板（支持变量替换）
     - 发送测试邮件

【邮件配置】

  1. 基础配置:
     - SMTP 服务器: smtp.qq.com（QQ邮箱）/ smtp.gmail.com（Gmail）
     - 端口: 587（推荐）或 465
     - 用户名: 你的邮箱地址
     - 密码/授权码: QQ邮箱在设置中获取授权码

  2. 收件人:
     - 单个: user@example.com
     - 多个: user1@qq.com, user2@qq.com, user3@gmail.com

  3. 模板变量（主题和内容都支持）:
     - {script} - 脚本名称
     - {time}   - 执行时间（HH:MM:SS）
     - {date}   - 执行日期（YYYY-MM-DD）
     - {result} - 脚本输出结果

  4. 模板示例:
     主题: TaskMod 通知 - {script}
     内容: 脚本 {script} 已于 {date} {time} 执行完成
     
           输出结果:
           {result}

  5. 启用通知:
     - 勾选"启用脚本执行完成后自动发送邮件通知"
     - 保存配置后，脚本执行完成会自动发送邮件

【截图功能】

  截图保存位置: /sdcard/TaskMod/screenshots/
  文件名格式: YYYYMMDD_HHmmss.png
  可通过 Web 面板拍摄或使用命令:
  screencap -p /sdcard/TaskMod/screenshots/$(date +%Y%m%d_%H%M%S).png

【脚本编写】

  脚本放在 /sdcard/TaskMod/scripts/ 目录下
  脚本中直接写 shell 命令即可

  示例 midea.sh:
    #!/system/bin/sh
    input keyevent 26
    sleep 2
    input swipe 540 1800 540 800 300
    sleep 1
    input tap 540 960

【常用命令】

  唤醒屏幕:       input keyevent 26
  锁屏:           input keyevent 224
  返回:           input keyevent 4
  Home:           input keyevent 3
  最近任务:       input keyevent 187
  点击屏幕:       input tap X Y
  滑动屏幕:       input swipe X1 Y1 X2 Y2 时间ms
  输入文本:       input text "内容"
  启动应用:       am start -n 包名/Activity
  启动服务:       am startservice -n 包名/Service
  强制停止:       am force-stop 包名
  截图:           screencap -p /path/to/save.png
  录屏:           screenrecord /path/to/save.mp4
  查看进程:       ps -A | grep 关键词
  查看设备信息:   getprop ro.product.model

【注意事项】

  - 修改配置后无需重启, 30秒内自动生效
  - 以#开头的行为注释
  - 脚本文件需要有执行权限
  - Web服务默认监听 0.0.0.0:9527
  - 邮件通知需要先配置并启用才会生效
