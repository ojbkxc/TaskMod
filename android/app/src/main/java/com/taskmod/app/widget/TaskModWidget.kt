package com.taskmod.app.widget

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.widget.RemoteViews
import com.taskmod.app.MainActivity
import com.taskmod.app.R
import com.taskmod.app.ServerManager
import com.taskmod.app.TaskModApp

class TaskModWidget : AppWidgetProvider() {

    override fun onUpdate(context: Context, appWidgetManager: AppWidgetManager, appWidgetIds: IntArray) {
        for (appWidgetId in appWidgetIds) {
            updateAppWidget(context, appWidgetManager, appWidgetId)
        }
    }

    override fun onReceive(context: Context, intent: Intent) {
        super.onReceive(context, intent)
        if (intent.action == "com.taskmod.STATUS_CHANGED") {
            val manager = AppWidgetManager.getInstance(context)
            val ids = manager.getAppWidgetIds(ComponentName(context, TaskModWidget::class.java))
            for (id in ids) {
                updateAppWidget(context, manager, id)
            }
        }
    }

    companion object {
        fun updateAppWidget(context: Context, appWidgetManager: AppWidgetManager, appWidgetId: Int) {
            val views = RemoteViews(context.packageName, R.layout.widget_layout)
            val manager = ServerManager(context)
            val running = manager.isRunning()

            views.setTextViewText(R.id.widget_status_text, if (running) "运行中" else "已停止")
            views.setInt(R.id.widget_status_dot, "setBackgroundResource",
                if (running) R.drawable.status_dot_running else R.drawable.status_dot_stopped)
            views.setTextViewText(R.id.widget_address,
                if (running) manager.getLanUrl() else "http://--:${manager.port}")

            // 点击打开主界面
            val intent = Intent(context, MainActivity::class.java)
            val pending = PendingIntent.getActivity(context, 0, intent,
                PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE)
            views.setOnClickPendingIntent(R.id.widget_status_dot, pending)

            appWidgetManager.updateAppWidget(appWidgetId, views)
        }
    }
}
