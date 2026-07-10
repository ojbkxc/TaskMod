#!/system/bin/sh
MODDIR=${0%/*}
SCHEDULE_FILE="/sdcard/TaskMod/schedule.conf"
SCRIPTS_DIR="/sdcard/TaskMod/scripts"
SCREENSHOTS_DIR="/sdcard/TaskMod/screenshots"
MARKER_DIR="$MODDIR/markers"
LOG_FILE="$MODDIR/TaskMod.log"
SERVER_BIN="$MODDIR/bin/taskmod-server"
export PATH=/sbin:/system/sbin:/system/bin:/system/xbin

log() {
  echo "$(date '+%Y-%m-%d %H:%M:%S') $1" >> "$LOG_FILE"
}

# 启动 Web 管理服务
start_web_server() {
  if [ -f "$SERVER_BIN" ]; then
    chmod 755 "$SERVER_BIN"
    mkdir -p "$SCREENSHOTS_DIR"
    nohup "$SERVER_BIN" >> "$LOG_FILE" 2>&1 &
    log "Web 管理服务已启动 (端口 9527)"
  else
    log "Web 服务二进制不存在: $SERVER_BIN"
  fi
}

# 等待系统完全启动
until [ "$(getprop sys.boot_completed)" = "1" ]; do
  sleep 10
done

mkdir -p "$MARKER_DIR"
mkdir -p "/sdcard/TaskMod/scripts"
mkdir -p "/sdcard/TaskMod/workflows"
if [ ! -f "$SCHEDULE_FILE" ]; then
  cp "$MODDIR/schedule.conf" "$SCHEDULE_FILE" 2>/dev/null
fi
if [ ! -f "$SCRIPTS_DIR/midea.sh" ]; then
  cp "$MODDIR/scripts/midea.sh" "$SCRIPTS_DIR/midea.sh" 2>/dev/null
fi

log "=== service.sh 启动 ==="

# 启动 Web 管理服务
start_web_server

match_today() {
  weeks="$1"
  today_dow=$(date +%u)
  case ",$weeks," in
    *",$today_dow,"*) return 0 ;;
  esac
  return 1
}

match_minute() {
  target_min=$1
  now_min=$(date +%H%M)
  if [ "$now_min" = "$target_min" ]; then
    return 0
  fi
  return 1
}

run_script() {
  script="$1"
  if [ -f "$SCRIPTS_DIR/$script" ]; then
    chmod 700 "$SCRIPTS_DIR/$script"
    /system/bin/sh "$SCRIPTS_DIR/$script" &
    log "执行脚本: $SCRIPTS_DIR/$script"
  else
    log "脚本不存在: $SCRIPTS_DIR/$script"
  fi
}

process_schedule() {
  while IFS= read -r line || [ -n "$line" ]; do
    line=$(echo "$line" | tr -d '\r')
    case "$line" in ""|\#*) continue ;; esac

    # 支持两种格式：管道分隔（Rust后端写入）和空格分隔（旧格式）
    case "$line" in
      *\|*)
        # 管道分隔格式: time|weeks|script|task_type|interval
        OLD_IFS="$IFS"
        IFS='|'
        set -- $line
        IFS="$OLD_IFS"
        time_val="$1"
        weeks="$2"
        script="$3"
        task_type="$4"
        interval="$5"

        if [ "$task_type" = "interval" ] && [ -n "$interval" ]; then
          now_min=$(date +%M)
          if [ $((now_min % interval)) -eq 0 ]; then
            marker="$MARKER_DIR/$(date +%Y%m%d%H%M)_$(echo "$script" | tr '/' '_')"
            if [ ! -f "$marker" ]; then
              touch "$marker"
              run_script "$script"
            fi
          fi
        else
          time_key=$(echo "$time_val" | tr -d ':')
          if match_minute "$time_key" && match_today "$weeks"; then
            marker="$MARKER_DIR/$(date +%Y%m%d)_${time_key}_${weeks}_$(echo "$script" | tr '/' '_')"
            if [ ! -f "$marker" ]; then
              touch "$marker"
              run_script "$script"
            fi
          fi
        fi
        ;;
      *)
        # 空格分隔旧格式（兼容）
        set -- $line
        type="$1"
        if [ "$type" = "every" ]; then
          mins="$2"
          script="$3"
          now_min=$(date +%M)
          if [ $((now_min % mins)) -eq 0 ]; then
            marker="$MARKER_DIR/$(date +%Y%m%d%H%M)_$(echo "$script" | tr '/' '_')"
            if [ ! -f "$marker" ]; then
              touch "$marker"
              run_script "$script"
            fi
          fi
        else
          time_val="$1"
          time_key=$(echo "$time_val" | tr -d ':')
          if [ "$#" -eq 3 ]; then
            weeks="$2"
            script="$3"
          else
            weeks="1,2,3,4,5,6,7"
            script="$2"
          fi
          if match_minute "$time_key" && match_today "$weeks"; then
            marker="$MARKER_DIR/$(date +%Y%m%d)_${time_key}_${weeks}_$(echo "$script" | tr '/' '_')"
            if [ ! -f "$marker" ]; then
              touch "$marker"
              run_script "$script"
            fi
          fi
        fi
        ;;
    esac
  done < "$SCHEDULE_FILE"
}

while true; do
  if [ -f "$SCHEDULE_FILE" ]; then
    process_schedule
  fi
  sleep 30
done
