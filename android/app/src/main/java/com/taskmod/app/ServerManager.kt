package com.taskmod.app

import android.content.Context
import android.util.Log
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

class ServerManager(private val context: Context) {

    companion object {
        private const val TAG = "ServerManager"
        private const val BINARY_NAME = "taskmod-server"
    }

    private var process: Process? = null
    private val binaryFile: File
        get() = File(context.filesDir, BINARY_NAME)

    /** 获取当前配置的端口 */
    val port: Int get() = ConfigManager.getPort()

    enum class ServerState {
        STOPPED, STARTING, RUNNING, ERROR
    }

    var state: ServerState = ServerState.STOPPED
        private set

    var lastError: String = ""
        private set

    fun prepare(): Boolean {
        // 从 assets 复制二进制文件
        if (!binaryFile.exists()) {
            try {
                context.assets.open(BINARY_NAME).use { input ->
                    binaryFile.outputStream().use { output ->
                        input.copyTo(output)
                    }
                }
                // 设置执行权限
                Runtime.getRuntime().exec(arrayOf("chmod", "755", binaryFile.absolutePath)).waitFor()
                Log.i(TAG, "二进制文件已复制: ${binaryFile.absolutePath}")
            } catch (e: Exception) {
                // assets 中没有二进制文件（开发阶段），尝试从 Magisk 模块目录复制
                val magiskBinary = File("/data/adb/modules/TaskMod/bin/arm64/$BINARY_NAME")
                if (magiskBinary.exists()) {
                    magiskBinary.copyTo(binaryFile, overwrite = true)
                    Runtime.getRuntime().exec(arrayOf("chmod", "755", binaryFile.absolutePath)).waitFor()
                    Log.i(TAG, "从 Magisk 模块复制二进制文件")
                } else {
                    lastError = "未找到二进制文件"
                    state = ServerState.ERROR
                    return false
                }
            }
        }
        return true
    }

    fun start(): Boolean {
        if (state == ServerState.RUNNING) return true
        if (!prepare()) return false

        state = ServerState.STARTING
        try {
            val dataDir = File(context.filesDir, "data")
            dataDir.mkdirs()

            val builder = ProcessBuilder(binaryFile.absolutePath)
            builder.directory(context.filesDir)
            builder.environment()["TMPDIR"] = context.cacheDir.absolutePath
            builder.redirectErrorStream(true)

            // 通过环境变量传递端口给服务端
            builder.environment()["TASKMOD_PORT"] = port.toString()

            process = builder.start()

            // 等待启动
            Thread.sleep(2000)

            if (isRunning()) {
                state = ServerState.RUNNING
                Log.i(TAG, "服务已启动，端口: $port")
                return true
            } else {
                lastError = "进程启动后立即退出"
                state = ServerState.ERROR
                return false
            }
        } catch (e: Exception) {
            lastError = e.message ?: "启动失败"
            state = ServerState.ERROR
            Log.e(TAG, "启动失败", e)
            return false
        }
    }

    fun stop() {
        try {
            process?.destroy()
            process = null
            // 确保进程被杀死
            Runtime.getRuntime().exec(arrayOf("pkill", "-f", BINARY_NAME)).waitFor()
        } catch (e: Exception) {
            Log.e(TAG, "停止失败", e)
        }
        state = ServerState.STOPPED
    }

    fun isRunning(): Boolean {
        // 先检查进程是否存活
        val proc = process
        if (proc != null) {
            try {
                proc.exitValue()
                // 如果能获取退出值，说明进程已结束
                process = null
                state = ServerState.STOPPED
                return false
            } catch (e: IllegalThreadStateException) {
                // 进程仍在运行
            }
        }

        // 通过 HTTP 检查服务是否响应
        return try {
            val url = URL("http://127.0.0.1:$port/api/status")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 2000
            conn.readTimeout = 2000
            val result = conn.responseCode == 200
            conn.disconnect()
            if (result && state != ServerState.RUNNING) {
                state = ServerState.RUNNING
            }
            result
        } catch (e: Exception) {
            false
        }
    }

    fun getLocalUrl(): String {
        return "http://127.0.0.1:$port"
    }

    fun getLanUrl(): String {
        return ConfigManager.getAccessUrl()
    }

    fun getAllAccessUrls(): List<String> {
        val urls = mutableListOf<String>()
        urls.add("本地: http://127.0.0.1:$port")
        for (info in NetworkHelper.getAllIps()) {
            urls.add("${info.type}: http://${info.ip}:$port")
        }
        val config = ConfigManager.load()
        if (config.customUrl.isNotBlank()) {
            urls.add("自定义: ${config.customUrl}")
        } else if (config.customIp.isNotBlank()) {
            urls.add("自定义: http://${config.customIp}:$port")
        }
        return urls
    }

    fun executeCommand(command: String): Pair<Boolean, String> {
        return RootHelper.executeRoot(command)
    }

    fun executeRoot(command: String): Pair<Boolean, String> {
        return RootHelper.executeRoot(command)
    }
}
