SKIPMOUNT=true
PROPFILE=false
POSTFSDATA=false
LATESTARTSERVICE=true

set_perm_recursive $MODPATH 0 0 0755 0755

# 创建目录
mkdir -p /sdcard/TaskMod/scripts
mkdir -p /sdcard/TaskMod/screenshots
mkdir -p /sdcard/TaskMod/workflows

# 复制配置和脚本
cp $MODPATH/sdcard/TaskMod/schedule.conf /sdcard/TaskMod/schedule.conf
cp $MODPATH/sdcard/TaskMod/email.conf /sdcard/TaskMod/email.conf 2>/dev/null
cp $MODPATH/sdcard/TaskMod/mqtt.conf /sdcard/TaskMod/mqtt.conf 2>/dev/null
cp $MODPATH/sdcard/TaskMod/scripts/midea.sh /sdcard/TaskMod/scripts/midea.sh

# 复制 Web 服务二进制
if [ -f "$MODPATH/bin/arm64/taskmod-server" ]; then
    cp $MODPATH/bin/arm64/taskmod-server $MODPATH/bin/taskmod-server
else
    ui_print "! Web服务二进制不存在"
    ui_print "! Web管理功能将不可用"
fi

# 设置权限
chmod 644 /sdcard/TaskMod/schedule.conf
chmod 755 /sdcard/TaskMod/scripts/midea.sh
chmod 755 $MODPATH/bin/taskmod-server 2>/dev/null

ui_print "----------------------------------"
ui_print "  TaskMod $(grep_prop version $MODPATH/module.prop) 安装成功"
ui_print "  定时任务管理 + Web面板"
ui_print "----------------------------------"
ui_print "配置文件：/sdcard/TaskMod/schedule.conf"
ui_print "脚本目录：/sdcard/TaskMod/scripts/"
ui_print "截图目录：/sdcard/TaskMod/screenshots/"
ui_print "管理面板：http://localhost:9527"
ui_print "修改配置后无需重启, 30秒内自动生效"
ui_print "----------------------------------"
