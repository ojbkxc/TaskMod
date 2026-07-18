package com.taskmod.app.widget

import android.app.AlarmManager
import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.os.SystemClock
import android.widget.RemoteViews
import android.widget.Toast
import com.taskmod.app.MainActivity
import com.taskmod.app.R
import com.taskmod.app.ServerManager
import com.taskmod.app.TaskModService

class TaskModWidget : AppWidgetProvider() {

    companion object {
        private const val ACTION_TOGGLE = "com.taskmod.widget.TOGGLE"
        private const val ACTION_UPDATE = "com.taskmod.widget.UPDATE"

        /**
         * 异步更新 Widget，避免 isRunning() 阻塞主线程
         */
        fun updateAppWidgetAsync(
            context: Context,
            appWidgetManager: AppWidgetManager,
            appWidgetId: Int
        ) {
            Thread {
                val manager = ServerManager.getInstance(context.applicationContext)
                val running = manager.isRunning()
                val views = RemoteViews(context.packageName, R.layout.widget_layout)

                views.setTextViewText(R.id.widget_status_text, if (running) "运行中" else "已停止")
                views.setInt(
                    R.id.widget_status_dot, "setBackgroundResource",
                    if (running) R.drawable.status_dot_running else R.drawable.status_dot_stopped
                )
                views.setTextViewText(
                    R.id.widget_address,
                    if (running) manager.getLanUrl() else "http://--:${manager.port}"
                )

                // 点击状态指示器切换服务
                val toggleIntent = Intent(context, TaskModWidget::class.java).apply {
                    action = ACTION_TOGGLE
                }
                val togglePending = PendingIntent.getBroadcast(
                    context, 0, toggleIntent,
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                )
                views.setOnClickPendingIntent(R.id.widget_status_dot, togglePending)

                // 点击打开主界面
                val openIntent = Intent(context, MainActivity::class.java)
                val openPending = PendingIntent.getActivity(
                    context, 1, openIntent,
                    PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
                )
                views.setOnClickPendingIntent(R.id.widget_status_text, openPending)

                appWidgetManager.updateAppWidget(appWidgetId, views)
            }.start()
        }

        /**
         * 启动定时刷新（每 30 秒），防止服务意外崩溃后 Widget 状态过时
         */
        fun schedulePeriodicUpdate(context: Context) {
            val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
            val intent = Intent(context, TaskModWidget::class.java).apply {
                action = ACTION_UPDATE
            }
            val pendingIntent = PendingIntent.getBroadcast(
                context, 0, intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
            )
            try {
                alarmManager.setInexactRepeating(
                    AlarmManager.ELAPSED_REALTIME,
                    SystemClock.elapsedRealtime() + 30000,
                    30000,
                    pendingIntent
                )
            } catch (e: SecurityException) {
                // 某些 ROM 可能限制 AlarmManager
            }
        }
    }

    override fun onUpdate(context: Context, appWidgetManager: AppWidgetManager, appWidgetIds: IntArray) {
        for (appWidgetId in appWidgetIds) {
            updateAppWidgetAsync(context, appWidgetManager, appWidgetId)
        }
        schedulePeriodicUpdate(context)
    }

    override fun onReceive(context: Context, intent: Intent) {
        super.onReceive(context, intent)
        when (intent.action) {
            ACTION_TOGGLE -> {
                Thread {
                    val manager = ServerManager.getInstance(context.applicationContext)
                    if (manager.isRunning()) {
                        TaskModService.stop(context)
                    } else {
                        TaskModService.start(context)
                    }
                    // 延迟更新 Widget 等待状态变化
                    Thread.sleep(1000)
                    val appWidgetManager = AppWidgetManager.getInstance(context)
                    val ids = appWidgetManager.getAppWidgetIds(ComponentName(context, TaskModWidget::class.java))
                    for (id in ids) {
                        updateAppWidgetAsync(context, appWidgetManager, id)
                    }
                }.start()
            }
            ACTION_UPDATE, "com.taskmod.STATUS_CHANGED" -> {
                val manager = AppWidgetManager.getInstance(context)
                val ids = manager.getAppWidgetIds(ComponentName(context, TaskModWidget::class.java))
                for (id in ids) {
                    updateAppWidgetAsync(context, manager, id)
                }
            }
        }
    }

    override fun onEnabled(context: Context) {
        super.onEnabled(context)
        schedulePeriodicUpdate(context)
    }

    override fun onDisabled(context: Context) {
        val alarmManager = context.getSystemService(Context.ALARM_SERVICE) as AlarmManager
        val intent = Intent(context, TaskModWidget::class.java).apply {
            action = ACTION_UPDATE
        }
        val pendingIntent = PendingIntent.getBroadcast(
            context, 0, intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )
        alarmManager.cancel(pendingIntent)
        super.onDisabled(context)
    }
}