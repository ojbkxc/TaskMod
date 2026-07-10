package com.taskmod.app

import android.app.Activity
import android.view.View
import android.widget.TextView
import android.widget.Toast
import com.google.android.material.button.MaterialButton
import kotlinx.coroutines.*

/**
 * 守护进程控制助手
 *
 * 简化在 Activity 中集成守护进程控制功能
 * 提供 UI 更新和用户交互处理
 */
class DaemonControlHelper(
    private val activity: Activity,
    private val scope: CoroutineScope
) {
    private val daemonManager = DaemonManager.getInstance(activity)
    private var statusCheckJob: Job? = null

    // UI 元素
    private var statusDot: View? = null
    private var tvStatus: TextView? = null
    private var tvPid: TextView? = null
    private var tvUptime: TextView? = null
    private var btnStart: MaterialButton? = null
    private var btnStop: MaterialButton? = null
    private var btnRestart: MaterialButton? = null

    /**
     * 初始化 UI 元素
     */
    fun initViews(
        statusDot: View,
        tvStatus: TextView,
        tvPid: TextView,
        tvUptime: TextView,
        btnStart: MaterialButton,
        btnStop: MaterialButton,
        btnRestart: MaterialButton
    ) {
        this.statusDot = statusDot
        this.tvStatus = tvStatus
        this.tvPid = tvPid
        this.tvUptime = tvUptime
        this.btnStart = btnStart
        this.btnStop = btnStop
        this.btnRestart = btnRestart

        setupListeners()
        startStatusCheck()
    }

    /**
     * 设置按钮点击监听
     */
    private fun setupListeners() {
        btnStart?.setOnClickListener {
            scope.launch(Dispatchers.IO) {
                val result = daemonManager.start()
                withContext(Dispatchers.Main) {
                    if (result.success) {
                        Toast.makeText(activity, "守护进程已启动", Toast.LENGTH_SHORT).show()
                        refreshStatus()
                    } else {
                        Toast.makeText(activity, "启动失败: ${result.error}", Toast.LENGTH_LONG).show()
                    }
                }
            }
        }

        btnStop?.setOnClickListener {
            scope.launch(Dispatchers.IO) {
                val result = daemonManager.stop()
                withContext(Dispatchers.Main) {
                    if (result.success) {
                        Toast.makeText(activity, "守护进程已停止", Toast.LENGTH_SHORT).show()
                        refreshStatus()
                    } else {
                        Toast.makeText(activity, "停止失败: ${result.error}", Toast.LENGTH_LONG).show()
                    }
                }
            }
        }

        btnRestart?.setOnClickListener {
            scope.launch(Dispatchers.IO) {
                val result = daemonManager.restart()
                withContext(Dispatchers.Main) {
                    if (result.success) {
                        Toast.makeText(activity, "守护进程已重启", Toast.LENGTH_SHORT).show()
                        // 等待重启完成后再刷新状态
                        delay(2000)
                        refreshStatus()
                    } else {
                        Toast.makeText(activity, "重启失败: ${result.error}", Toast.LENGTH_LONG).show()
                    }
                }
            }
        }
    }

    /**
     * 启动状态检查定时任务
     */
    private fun startStatusCheck() {
        statusCheckJob?.cancel()
        statusCheckJob = scope.launch(Dispatchers.IO) {
            while (isActive) {
                refreshStatus()
                delay(5000) // 每5秒检查一次
            }
        }
    }

    /**
     * 刷新状态显示
     */
    fun refreshStatus() {
        scope.launch(Dispatchers.IO) {
            val result = daemonManager.getStatus()

            withContext(Dispatchers.Main) {
                if (result.success && result.data != null) {
                    val status = result.data
                    updateUIRunning(status.pid, daemonManager.formatUptime(status.uptimeSeconds))
                } else {
                    updateUIStopped(result.error)
                }
            }
        }
    }

    /**
     * 更新 UI 为运行状态
     */
    private fun updateUIRunning(pid: Int, uptime: String) {
        statusDot?.setBackgroundResource(R.drawable.status_dot_running)
        tvStatus?.text = "运行中"
        tvStatus?.setTextColor(activity.getColor(R.color.success))
        tvPid?.text = "PID: $pid"
        tvUptime?.text = "运行时长: $uptime"

        btnStart?.isEnabled = false
        btnStop?.isEnabled = true
        btnRestart?.isEnabled = true
    }

    /**
     * 更新 UI 为停止状态
     */
    private fun updateUIStopped(error: String? = null) {
        statusDot?.setBackgroundResource(R.drawable.status_dot_stopped)
        tvStatus?.text = "未运行"
        tvStatus?.setTextColor(activity.getColor(R.color.text_secondary))
        tvPid?.text = ""
        tvUptime?.text = error ?: ""

        btnStart?.isEnabled = true
        btnStop?.isEnabled = false
        btnRestart?.isEnabled = false
    }

    /**
     * 停止状态检查
     */
    fun stopStatusCheck() {
        statusCheckJob?.cancel()
        statusCheckJob = null
    }

    /**
     * 释放资源
     */
    fun destroy() {
        stopStatusCheck()
    }
}