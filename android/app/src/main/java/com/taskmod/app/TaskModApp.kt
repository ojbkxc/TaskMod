package com.taskmod.app

import android.app.Application
import android.app.NotificationChannel
import android.app.NotificationManager
import android.os.Build

class TaskModApp : Application() {

    companion object {
        const val CHANNEL_ID = "taskmod_service"
        const val CHANNEL_NAME = "TaskMod 服务"
        const val PORT = 9527
        const val GITHUB_REPO = "ojbkxc/TaskMod"
    }

    override fun onCreate() {
        super.onCreate()
        createNotificationChannel()
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                CHANNEL_NAME,
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "TaskMod 后台服务通知"
                setShowBadge(false)
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager.createNotificationChannel(channel)
        }
    }
}
