package com.taskmod.app

import android.app.*
import android.content.Context
import android.content.Intent
import android.os.Build
import android.os.IBinder
import android.os.PowerManager
import android.util.Log
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.*

class TaskModService : Service(), CoroutineScope by CoroutineScope(Dispatchers.Default + SupervisorJob()) {

    companion object {
        private const val TAG = "TaskModService"
        private const val NOTIFICATION_ID = 1001

        const val ACTION_START = "com.taskmod.action.START"
        const val ACTION_STOP = "com.taskmod.action.STOP"
        const val ACTION_SCREENSHOT = "com.taskmod.action.SCREENSHOT"
        const val ACTION_UNLOCK = "com.taskmod.action.UNLOCK"
        const val ACTION_REBOOT = "com.taskmod.action.REBOOT"

        fun start(context: Context) {
            val intent = Intent(context, TaskModService::class.java).apply {
                action = ACTION_START
            }
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent)
            } else {
                context.startService(intent)
            }
        }

        fun stop(context: Context) {
            val intent = Intent(context, TaskModService::class.java).apply {
                action = ACTION_STOP
            }
            context.startService(intent)
        }
    }

    private lateinit var serverManager: ServerManager
    private var wakeLock: PowerManager.WakeLock? = null

    override fun onCreate() {
        super.onCreate()
        serverManager = ServerManager.getInstance(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        when (intent?.action) {
            ACTION_START -> handleStart()
            ACTION_STOP -> handleStop()
            ACTION_SCREENSHOT -> handleScreenshot()
            ACTION_UNLOCK -> handleUnlock()
            ACTION_REBOOT -> handleReboot()
        }
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    private fun handleStart() {
        startForeground(NOTIFICATION_ID, buildNotification("启动中…"))
        acquireWakeLock()

        launch {
            val success = serverManager.start()
            if (success) {
                updateNotification("运行中 - 端口 ${serverManager.port}")
                sendBroadcast(Intent("com.taskmod.STATUS_CHANGED").putExtra("running", true))
            } else {
                updateNotification("启动失败: ${serverManager.lastError}")
                sendBroadcast(Intent("com.taskmod.STATUS_CHANGED").putExtra("running", false))
            }
        }
    }

    private fun handleStop() {
        launch(Dispatchers.IO) {
            serverManager.stop()
            withContext(Dispatchers.Main) {
                releaseWakeLock()
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
                sendBroadcast(Intent("com.taskmod.STATUS_CHANGED").putExtra("running", false))
            }
        }
    }

    private fun handleScreenshot() {
        launch(Dispatchers.IO) {
            val (success, _) = serverManager.executeCommand("screencap -p /sdcard/screenshot.png")
            Log.i(TAG, "截屏: ${if (success) "成功" else "失败"}")
        }
    }

    private fun handleUnlock() {
        launch(Dispatchers.IO) {
            serverManager.executeCommand("input keyevent KEYCODE_WAKEUP")
            delay(300)
            serverManager.executeCommand("input swipe 540 1800 540 600 300")
            Log.i(TAG, "上滑解锁已执行")
        }
    }

    private fun handleReboot() {
        launch(Dispatchers.IO) {
            serverManager.executeCommand("reboot")
        }
    }

    private fun buildNotification(text: String): Notification {
        val openIntent = Intent(this, MainActivity::class.java)
        val openPending = PendingIntent.getActivity(
            this, 0, openIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val screenshotIntent = Intent(this, TaskModService::class.java).apply {
            action = ACTION_SCREENSHOT
        }
        val screenshotPending = PendingIntent.getService(
            this, 1, screenshotIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val unlockIntent = Intent(this, TaskModService::class.java).apply {
            action = ACTION_UNLOCK
        }
        val unlockPending = PendingIntent.getService(
            this, 2, unlockIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val stopIntent = Intent(this, TaskModService::class.java).apply {
            action = ACTION_STOP
        }
        val stopPending = PendingIntent.getService(
            this, 3, stopIntent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        return NotificationCompat.Builder(this, TaskModApp.CHANNEL_ID)
            .setContentTitle("TaskMod")
            .setContentText(text)
            .setSmallIcon(R.drawable.ic_server)
            .setContentIntent(openPending)
            .addAction(R.drawable.ic_camera, "截屏", screenshotPending)
            .addAction(R.drawable.ic_unlock, "解锁", unlockPending)
            .addAction(R.drawable.ic_server, "停止", stopPending)
            .setOngoing(true)
            .setSilent(true)
            .build()
    }

    private fun updateNotification(text: String) {
        val notification = buildNotification(text)
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, notification)
    }

    private fun acquireWakeLock() {
        val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(PowerManager.PARTIAL_WAKE_LOCK, "TaskMod::Server")
        wakeLock?.acquire(60 * 60 * 1000L) // 1小时

        // 安全超时：协程中延时后确保释放，防止异常情况泄漏
        launch {
            delay(60 * 60 * 1000L)
            releaseWakeLock()
        }
    }

    private fun releaseWakeLock() {
        wakeLock?.let {
            if (it.isHeld) it.release()
        }
        wakeLock = null
    }

    override fun onDestroy() {
        serverManager.stop()
        releaseWakeLock()
        cancel() // 取消所有协程
        super.onDestroy()
    }
}
