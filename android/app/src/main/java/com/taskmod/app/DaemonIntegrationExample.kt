package com.taskmod.app

import android.os.Bundle
import androidx.appcompat.app.AppCompatActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch

/**
 * 守护进程集成示例
 *
 * 展示如何在 Activity 中集成 cloudflared 隧道守护进程控制
 */
class DaemonIntegrationExample : AppCompatActivity() {

    private lateinit var daemonHelper: DaemonControlHelper

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        daemonHelper = DaemonControlHelper(this, lifecycleScope)
    }

    /**
     * 直接使用 DaemonManager 控制守护进程（无 UI）
     */
    private fun controlDaemonDirectly() {
        val daemonManager = DaemonManager.getInstance(this)

        lifecycleScope.launch(Dispatchers.IO) {
            // 查询状态
            val status = daemonManager.getStatus()
            if (status.success && status.data != null) {
                android.util.Log.i("Daemon", "守护进程运行中: PID=${status.data.pid}")
            }

            // 启动守护进程
            val result = daemonManager.start()
            android.util.Log.i("Daemon", if (result.success) "启动成功" else "启动失败: ${result.error}")

            // 停止守护进程
            daemonManager.stop()

            // 重启守护进程（热重载）
            daemonManager.restart()
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        daemonHelper.destroy()
    }
}