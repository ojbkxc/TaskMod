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

class TaskModService : Service() {

    companion object {
        private const val TAG = "TaskModService"
        private const val NOTIFICATION_ID = 1001

        const val ACTION_START = "com.taskmod.action.START"
        const val ACTION_STOP = "com.taskmod.action.STOP"
        const val ACTION_SCREENSHOT = "com.taskmod.action.SCREENSHOT"
        const val ACTION_UNLOCK = "com.taskmod.action.UNLOCK"
        const val ACTION_REBOOT = "com.taskmod.action.REBOOT"

        @Volatile
        private var instance: TaskModService? = null

        fun getInstance(): TaskModService? = instance

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

    private val serviceScope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private lateinit var serverManager: ServerManager
    private var wakeLock: PowerManager.WakeLock? = null
    private var wakeLockRenewJob: Job? = null

    @Volatile
    private var stopping = false

    override fun onCreate() {
        super.onCreate()
        instance = this
        Log.i(TAG, "onCreate")

        serverManager = ServerManager.getInstance(this)
        // START_STICKY 重建时，重置 ServerManager 状态以避免使用旧的 Process 引用
        serverManager.resetState()

        // 立即启动前台服务 — 必须在 onCreate 5 秒内调用
        startForeground(NOTIFICATION_ID, buildNotification("启动中…"))

        // 获取 WakeLock，防止 CPU 休眠
        acquireWakeLock()
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i(TAG, "onStartCommand: action=${intent?.action}")

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

    /**
     * 当用户从最近任务列表划掉时，自动重启服务
     */
    override fun onTaskRemoved(rootIntent: Intent?) {
        Log.i(TAG, "onTaskRemoved: 尝试重启服务")
        val restartIntent = Intent(this, TaskModService::class.java)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            startForegroundService(restartIntent)
        } else {
            startService(restartIntent)
        }
        super.onTaskRemoved(rootIntent)
    }

    private fun handleStart() {
        if (serverManager.state == ServerManager.ServerState.RUNNING) return
        updateNotification("启动中…")

        serviceScope.launch {
            val success = serverManager.start()
            if (success) {
                val accessUrl = ConfigManager.getAccessUrl()
                updateNotification("运行中 - $accessUrl")
                sendBroadcast(Intent("com.taskmod.STATUS_CHANGED").putExtra("running", true))
            } else {
                updateNotification("启动失败: ${serverManager.lastError}")
                sendBroadcast(Intent("com.taskmod.STATUS_CHANGED").putExtra("running", false))
            }
        }
    }

    private fun handleStop() {
        if (stopping) return
        stopping = true
        serviceScope.launch(Dispatchers.IO) {
            try {
                serverManager.stop()
            } finally {
                withContext(Dispatchers.Main) {
                    releaseWakeLock()
                    sendBroadcast(Intent("com.taskmod.STATUS_CHANGED").putExtra("running", false))
                    stopForeground(STOP_FOREGROUND_DETACH)
                    stopSelf()
                }
            }
        }
    }

    private fun handleScreenshot() {
        serviceScope.launch(Dispatchers.IO) {
            val (success, _) = serverManager.executeCommand("screencap -p /sdcard/screenshot.png")
            Log.i(TAG, "截屏: ${if (success) "成功" else "失败"}")
        }
    }

    private fun handleUnlock() {
        serviceScope.launch(Dispatchers.IO) {
            serverManager.executeCommand("input keyevent KEYCODE_WAKEUP")
            delay(300)
            serverManager.executeCommand("input swipe 540 1800 540 600 300")
            Log.i(TAG, "上滑解锁已执行")
        }
    }

    private fun handleReboot() {
        serviceScope.launch(Dispatchers.IO) {
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

    fun updateNotification(text: String) {
        val notification = buildNotification(text)
        val manager = getSystemService(NotificationManager::class.java)
        manager.notify(NOTIFICATION_ID, notification)
    }

    private fun acquireWakeLock() {
        val pm = getSystemService(Context.POWER_SERVICE) as PowerManager
        wakeLock = pm.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "TaskMod::ServerWakeLock"
        )
        // 非引用计数，10 小时超时
        wakeLock?.setReferenceCounted(false)
        wakeLock?.acquire(10 * 60 * 60 * 1000L)
        Log.i(TAG, "WakeLock acquired (10h timeout)")
    }

    private fun releaseWakeLock() {
        wakeLockRenewJob?.cancel()
        wakeLockRenewJob = null
        wakeLock?.let {
            if (it.isHeld) {
                it.release()
                Log.i(TAG, "WakeLock released")
            }
        }
        wakeLock = null
    }

    override fun onDestroy() {
        Log.i(TAG, "onDestroy")
        if (!stopping) {
            // 被系统杀死时清理资源
            serverManager.stop()
        }
        releaseWakeLock()
        serviceScope.cancel()
        instance = null
        super.onDestroy()
    }
}