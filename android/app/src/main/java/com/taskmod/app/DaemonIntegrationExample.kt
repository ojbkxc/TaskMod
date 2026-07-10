package com.taskmod.app

import android.os.Bundle
import android.view.LayoutInflater
import android.view.View
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import androidx.lifecycle.lifecycleScope
import com.google.android.material.button.MaterialButton

/**
 * 守护进程集成示例
 *
 * 展示如何在 Activity 中集成 cloudflared 隧道守护进程控制
 */
class DaemonIntegrationExample : AppCompatActivity() {

    private lateinit var daemonHelper: DaemonControlHelper

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 方法1: 使用独立布局文件
        // setContentView(R.layout.activity_with_daemon)

        // 方法2: 动态添加守护进程控制卡片
        setupDaemonControl()
    }

    /**
     * 方法1: 在 XML 布局中直接包含守护进程控制
     *
     * 在 activity_main.xml 或其他布局文件中添加:
     * <include layout="@layout/layout_daemon_control" />
     */
    private fun setupWithXmlLayout() {
        daemonHelper = DaemonControlHelper(this, lifecycleScope)

        // 初始化 UI 元素
        daemonHelper.initViews(
            statusDot = findViewById(R.id.daemon_status_dot),
            tvStatus = findViewById(R.id.tv_daemon_status),
            tvPid = findViewById(R.id.tv_daemon_pid),
            tvUptime = findViewById(R.id.tv_daemon_uptime),
            btnStart = findViewById(R.id.btn_daemon_start),
            btnStop = findViewById(R.id.btn_daemon_stop),
            btnRestart = findViewById(R.id.btn_daemon_restart)
        )
    }

    /**
     * 方法2: 动态创建守护进程控制 UI
     */
    private fun setupDaemonControl() {
        // 创建守护进程控制卡片
        val daemonCard = LayoutInflater.from(this).inflate(
            R.layout.layout_daemon_control,
            null
        )

        daemonHelper = DaemonControlHelper(this, lifecycleScope)

        // 初始化 UI 元素
        daemonHelper.initViews(
            statusDot = daemonCard.findViewById(R.id.daemon_status_dot),
            tvStatus = daemonCard.findViewById(R.id.tv_daemon_status),
            tvPid = daemonCard.findViewById(R.id.tv_daemon_pid),
            tvUptime = daemonCard.findViewById(R.id.tv_daemon_uptime),
            btnStart = daemonCard.findViewById(R.id.btn_daemon_start),
            btnStop = daemonCard.findViewById(R.id.btn_daemon_stop),
            btnRestart = daemonCard.findViewById(R.id.btn_daemon_restart)
        )

        // 添加到主布局
        // val mainLayout = findViewById<LinearLayout>(R.id.main_container)
        // mainLayout.addView(daemonCard, 0) // 添加到顶部
    }

    /**
     * 方法3: 使用 DaemonManager 直接控制（无 UI）
     */
    private fun controlDaemonDirectly() {
        val daemonManager = DaemonManager.getInstance(this)

        // 查询状态
        val status = daemonManager.getStatus()
        if (status.success && status.data != null) {
            println("守护进程运行中: PID=${status.data.pid}")
        }

        // 启动守护进程
        lifecycleScope.launch(kotlinx.coroutines.Dispatchers.IO) {
            val result = daemonManager.start()
            if (result.success) {
                println("启动成功")
            } else {
                println("启动失败: ${result.error}")
            }
        }

        // 停止守护进程
        lifecycleScope.launch(kotlinx.coroutines.Dispatchers.IO) {
            val result = daemonManager.stop()
            if (result.success) {
                println("已停止")
            }
        }

        // 重启守护进程（热重载）
        lifecycleScope.launch(kotlinx.coroutines.Dispatchers.IO) {
            val result = daemonManager.restart()
            if (result.success) {
                println("重启成功")
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        daemonHelper.destroy()
    }
}

/**
 * 使用示例: 在现有 Activity 中添加守护进程控制
 *
 * 步骤:
 * 1. 在布局 XML 中添加:
 *    <include layout="@layout/layout_daemon_control" />
 *
 * 2. 在 Activity 中初始化:
 *    private lateinit var daemonHelper: DaemonControlHelper
 *
 *    override fun onCreate(savedInstanceState: Bundle?) {
 *        super.onCreate(savedInstanceState)
 *        setContentView(R.layout.your_layout)
 *
 *        daemonHelper = DaemonControlHelper(this, lifecycleScope)
 *        daemonHelper.initViews(
 *            statusDot = findViewById(R.id.daemon_status_dot),
 *            tvStatus = findViewById(R.id.tv_daemon_status),
 *            tvPid = findViewById(R.id.tv_daemon_pid),
 *            tvUptime = findViewById(R.id.tv_daemon_uptime),
 *            btnStart = findViewById(R.id.btn_daemon_start),
 *            btnStop = findViewById(R.id.btn_daemon_stop),
 *            btnRestart = findViewById(R.id.btn_daemon_restart)
 *        )
 *    }
 *
 *    override fun onDestroy() {
 *        super.onDestroy()
 *        daemonHelper.destroy()
 *    }
 */