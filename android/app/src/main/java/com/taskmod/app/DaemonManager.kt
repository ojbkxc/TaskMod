package com.taskmod.app

import android.content.Context
import android.util.Log
import java.net.HttpURLConnection
import java.net.URL

/**
 * 守护进程管理 - 管理 cloudflared 隧道守护进程
 */
class DaemonManager private constructor(private val context: Context) {

    companion object {
        private const val TAG = "DaemonManager"
        private const val DEFAULT_PORT = 8888
        private const val BINARY_NAME = "cloudflared"
        private const val PID_FILE = "/data/local/tmp/cloudflared.pid"

        @Volatile
        private var instance: DaemonManager? = null

        fun getInstance(context: Context): DaemonManager {
            return instance ?: synchronized(this) {
                instance ?: DaemonManager(context.applicationContext).also { instance = it }
            }
        }
    }

    data class DaemonStatus(
        val pid: Int,
        val uptimeSeconds: Long,
        val isRunning: Boolean
    )

    data class OperationResult(
        val success: Boolean,
        val data: DaemonStatus? = null,
        val error: String? = null
    )

    private val binaryPath: String
        get() = "${context.filesDir}/$BINARY_NAME"

    fun start(): OperationResult {
        return try {
            // 检查是否已在运行
            val status = getStatus()
            if (status.success && status.data?.isRunning == true) {
                return OperationResult(true, status.data)
            }

            // 确保二进制存在
            ensureBinary()

            val cmd = "nohup $binaryPath tunnel --no-autoupdate --url http://127.0.0.1:${getPort()} > /dev/null 2>&1 & echo \$!"
            Log.i(TAG, "执行命令: $cmd")
            val (success, result) = executeRoot(cmd)

            if (success && result.isNotBlank()) {
                val pid = result.trim().toIntOrNull() ?: 0
                if (pid > 0) {
                    // 保存 PID
                    RootHelper.executeRoot("echo $pid > $PID_FILE")
                    Log.i(TAG, "守护进程已启动: PID=$pid")
                    return OperationResult(true, DaemonStatus(pid, 0, true))
                }
            }

            OperationResult(false, error = "启动失败: $result")
        } catch (e: Exception) {
            Log.e(TAG, "启动守护进程失败", e)
            OperationResult(false, error = e.message)
        }
    }

    fun stop(): OperationResult {
        return try {
            val status = getStatus()
            if (status.success && status.data != null) {
                executeRoot("kill -9 ${status.data.pid}")
                executeRoot("rm -f $PID_FILE")
                Log.i(TAG, "守护进程已停止: PID=${status.data.pid}")
                OperationResult(true)
            } else {
                OperationResult(true, error = "守护进程未运行")
            }
        } catch (e: Exception) {
            Log.e(TAG, "停止守护进程失败", e)
            OperationResult(false, error = e.message)
        }
    }

    fun restart(): OperationResult {
        val stopResult = stop()
        if (!stopResult.success) {
            return stopResult
        }
        // 等待进程完全退出
        Thread.sleep(1000)
        return start()
    }

    fun getStatus(): OperationResult {
        return try {
            // 使用 RootHelper 查找进程
            val (success, result) = executeRoot("cat $PID_FILE 2>/dev/null")
            if (!success || result.isBlank()) {
                return OperationResult(true, DaemonStatus(0, 0, false))
            }

            val pid = result.trim().toIntOrNull() ?: 0
            if (pid <= 0) {
                return OperationResult(true, DaemonStatus(0, 0, false))
            }

            // 检查进程是否存活（toybox ps 即使 PID 不存在也返回 0，需检查输出）
            val (_, psResult) = executeRoot("ps -p $pid -o pid= 2>/dev/null")
            if (psResult.trim().isBlank()) {
                return OperationResult(true, DaemonStatus(0, 0, false))
            }

            // 获取运行时间
            val (_, uptimeResult) = executeRoot("ps -p $pid -o etime= 2>/dev/null")
            val uptime = if (uptimeResult.isNotBlank()) parseUptime(uptimeResult.trim()) else 0L

            OperationResult(true, DaemonStatus(pid, uptime, true))
        } catch (e: Exception) {
            Log.e(TAG, "获取守护进程状态失败", e)
            OperationResult(false, error = e.message)
        }
    }

    fun makeRequest(endpoint: String): String? {
        return try {
            val url = URL("http://127.0.0.1:$DEFAULT_PORT$endpoint")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 5000
            conn.readTimeout = 5000
            conn.setRequestProperty("Connection", "close")

            val responseCode = conn.responseCode
            if (responseCode in 200..299) {
                conn.inputStream.bufferedReader().use { it.readText() }
            } else {
                Log.w(TAG, "请求失败: $responseCode")
                null
            }
        } catch (e: Exception) {
            Log.e(TAG, "请求失败: $endpoint", e)
            null
        }
    }

    fun formatUptime(seconds: Long): String {
        if (seconds <= 0) return "刚刚启动"
        val hours = seconds / 3600
        val minutes = (seconds % 3600) / 60
        val secs = seconds % 60
        return buildString {
            if (hours > 0) append("${hours}小时")
            if (minutes > 0) append("${minutes}分钟")
            append("${secs}秒")
        }
    }

    private fun parseUptime(uptimeStr: String): Long {
        if (uptimeStr.isBlank()) return 0
        // 格式如 "01:23:45" 或 "1-01:23:45"
        return try {
            val parts = uptimeStr.split("-")
            if (parts.size == 2) {
                val days = parts[0].toLong()
                val timeParts = parts[1].split(":")
                val hours = timeParts[0].toLong()
                val minutes = timeParts[1].toLong()
                val seconds = timeParts[2].toLong()
                days * 86400 + hours * 3600 + minutes * 60 + seconds
            } else {
                val timeParts = uptimeStr.split(":")
                when (timeParts.size) {
                    3 -> timeParts[0].toLong() * 3600 + timeParts[1].toLong() * 60 + timeParts[2].toLong()
                    2 -> timeParts[0].toLong() * 60 + timeParts[1].toLong()
                    else -> 0
                }
            }
        } catch (e: Exception) {
            0
        }
    }

    private fun executeRoot(command: String): Pair<Boolean, String> {
        return RootHelper.executeRoot(command)
    }

    private fun getPort(): Int {
        // 优先使用 TaskMod 服务端口（cloudflared 需要隧道到该端口）
        return try {
            ConfigManager.load().port
        } catch (e: Exception) {
            DEFAULT_PORT
        }
    }

    private fun ensureBinary() {
        val binary = java.io.File(binaryPath)
        if (!binary.exists()) {
            try {
                context.assets.open(BINARY_NAME).use { input ->
                    binary.outputStream().use { output ->
                        input.copyTo(output)
                    }
                }
                RootHelper.executeRoot("chmod 755 $binaryPath")
                Log.i(TAG, "二进制文件已复制: $binaryPath")
            } catch (e: Exception) {
                Log.w(TAG, "二进制文件不存在于 assets 中: $BINARY_NAME")
            }
        }
    }
}