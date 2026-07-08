==============================
      TaskMod v4.0 使用说明
==============================

【目录结构】
  /sdcard/TaskMod/
  ├── schedule.conf        -- 定时任务配置
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

  启动后访问: http://设备IP:8080
  功能:
  - 任务管理: 查看/添加/删除定时任务
  - 日志查看: 实时查看执行日志
  - 截图管理: 拍摄/查看/删除截图
  - 脚本管理: 在线编辑脚本
  - 配置编辑: 在线编辑 schedule.conf
  - 手动触发: 一键执行脚本
  - 邮件通知: 发送邮件通知

【截图功能】

  截图保存位置: /sdcard/TaskMod/screenshots/
  文件名格式: YYYYMMDD_HHmmss.png
  可通过 Web 面板拍摄或使用命令:
  screencap -p /sdcard/TaskMod/screenshots/$(date +%Y%m%d_%H%M%S).png

【脚本编写】

  脚本放在 /sdcard/TaskMod/scripts/ 目录下
  脚本中直接写 adb shell 命令即可

  示例 midea.sh:
    input keyevent 26
    sleep 2
    input swipe 540 1800 540 800 300
    sleep 1
    input tap 540 960

【常用命令】

  唤醒屏幕:       input keyevent 26
  返回:           input keyevent 4
  Home:           input keyevent 3
  点击屏幕:       input tap X Y
  滑动屏幕:       input swipe X1 Y1 X2 Y2 时间ms
  启动应用:       am start -n 包名/Activity
  启动服务:       am startservice -n 包名/Service

【邮件配置示例】

  SMTP服务器: smtp.qq.com
  端口: 587
  用户名: your@qq.com
  授权码: (在QQ邮箱设置中获取)

【注意事项】

  - 修改配置后无需重启, 30秒内自动生效
  - 以#开头的行为注释
  - 脚本文件需要有执行权限
  - Web服务默认监听 0.0.0.0:8080
