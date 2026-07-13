package com.taskmod.app

import android.content.Context
import android.util.Log
import java.io.BufferedReader
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

class ServerManager private constructor(private val context: Context) {

    companion object {
        private const val TAG = "ServerManager"
        private const val BINARY_NAME = "taskmod-server"

        @Volatile
        private var instance: ServerManager? = null

        fun getInstance(context: Context): ServerManager {
            return instance ?: synchronized(this) {
                instance ?: ServerManager(context.applicationContext).also { instance = it }
            }
        }
    }

    private var process: Process? = null
    private val binaryFile: File
        get() = File(context.filesDir, BINARY_NAME)

    /** 获取当前配置的端口 */
    val port: Int get() = ConfigManager.getPort()

    enum class ServerState {
        STOPPED, STARTING, RUNNING, ERROR
    }

    @Volatile
    var state: ServerState = ServerState.STOPPED
        private set

    @Volatile
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

    /**
     * 检查端口是否已被占用（Magisk service.sh 可能已启动了服务）
     */
    private fun isPortInUse(port: Int): Boolean {
        return try {
            val url = URL("http://127.0.0.1:$port/api/status")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 300
            conn.readTimeout = 500
            conn.requestMethod = "HEAD"
            conn.setRequestProperty("Connection", "close")
            val code = conn.responseCode
            conn.disconnect()
            code == 200
        } catch (e: Exception) {
            false
        }
    }

    /**
     * 检查是否已有同名的进程在运行
     */
    private fun isProcessAlive(): Boolean {
        return try {
            val p = Runtime.getRuntime().exec(arrayOf("pgrep", "-f", BINARY_NAME))
            val exitCode = p.waitFor()
            exitCode == 0
        } catch (e: Exception) {
            false
        }
    }

    fun start(): Boolean {
        // 如果已经在运行（HTTP 检查），直接复用，不重复启动
        if (isPortInUse(port)) {
            Log.i(TAG, "端口 $port 已有服务在运行（可能由 Magisk 启动），直接复用")
            state = ServerState.RUNNING
            return true
        }

        if (state == ServerState.RUNNING) return true
        if (!prepare()) return false

        // 如果有残留进程但端口不通，说明进程卡死了，先杀掉
        if (isProcessAlive()) {
            Log.w(TAG, "发现残留进程但端口不通，先清理")
            killAllProcesses()
        }

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

            // 轮询等待服务就绪（最多 5 秒，每 200ms 检查一次）
            val maxWait = 5000L
            val interval = 200L
            var waited = 0L
            while (waited < maxWait) {
                Thread.sleep(interval)
                waited += interval
                if (isPortInUse(port)) break
            }

            if (isPortInUse(port)) {
                state = ServerState.RUNNING
                Log.i(TAG, "服务已启动，端口: $port")
                return true
            } else {
                // 进程已退出，尝试读取输出日志
                var outputLog = ""
                try {
                    process?.inputStream?.bufferedReader()?.use { reader ->
                        outputLog = reader.readText().take(500)
                    }
                } catch (_: Exception) {}
                lastError = "进程启动后立即退出${if (outputLog.isNotBlank()) ": $outputLog" else ""}"
                state = ServerState.ERROR
                process = null
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
            // 先尝试优雅停止
            process?.destroy()
            process = null
            // 等待一小段时间让进程退出
            Thread.sleep(200)
            // 如果还有残留，强制杀死
            if (isProcessAlive()) {
                killAllProcesses()
            }
        } catch (e: Exception) {
            Log.e(TAG, "停止失败", e)
        }
        state = ServerState.STOPPED
    }

    /**
     * 杀死所有同名进程（不使用 pkill -f，避免误杀）
     */
    private fun killAllProcesses() {
        try {
            // 先用 pgrep 找到 PID，再逐个 kill
            val pgrep = Runtime.getRuntime().exec(arrayOf("pgrep", "-f", BINARY_NAME))
            val pids = pgrep.inputStream.bufferedReader().readLines().map { it.trim() }.filter { it.isNotBlank() }
            pgrep.waitFor()

            for (pid in pids) {
                try {
                    Log.i(TAG, "杀死进程 PID: $pid")
                    Runtime.getRuntime().exec(arrayOf("kill", "-9", pid)).waitFor()
                } catch (e: Exception) {
                    Log.w(TAG, "杀死 PID $pid 失败: $e")
                }
            }
        } catch (e: Exception) {
            // fallback: 使用 pkill
            Log.w(TAG, "pgrep 方式失败，fallback 到 pkill")
            try {
                Runtime.getRuntime().exec(arrayOf("pkill", "-9", "-f", BINARY_NAME)).waitFor()
            } catch (ex: Exception) {
                Log.e(TAG, "pkill 也失败: $ex")
            }
        }
    }

    @Volatile
    private var lastCheckTime: Long = 0
    @Volatile
    private var lastCheckResult: Boolean = false

    fun isRunning(): Boolean {
        val now = System.currentTimeMillis()
        // 缓存: 仅当上次结果是 true 且在 3 秒内时快速返回
        if (now - lastCheckTime < 3000 && lastCheckResult && state == ServerState.RUNNING) {
            return true
        }

        // 如果进程对象存在，先检查是否还活着
        val proc = process
        if (proc != null) {
            try {
                proc.exitValue()
                // 进程已退出
                process = null
                // 不要立即设为 STOPPED，可能由 Magisk 重启了，再检查一次端口
            } catch (_: IllegalThreadStateException) {
                // 进程还活着
            }
        }

        lastCheckTime = now

        // 用 HTTP HEAD 检查端口是否在监听（最可靠的判断）
        return try {
            val url = URL("http://127.0.0.1:$port/api/status")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 500
            conn.readTimeout = 1000
            conn.requestMethod = "HEAD"
            conn.setRequestProperty("Connection", "close")
            val result = conn.responseCode == 200
            conn.disconnect()

            if (result) {
                if (state != ServerState.RUNNING) {
                    state = ServerState.RUNNING
                    Log.i(TAG, "检测到服务已在运行（可能由外部启动）")
                }
                lastCheckResult = true
            } else {
                if (proc == null && state == ServerState.RUNNING) {
                    // HTTP 检查失败且没有本地进程对象，说明服务已停止
                    state = ServerState.STOPPED
                    Log.i(TAG, "检测到服务已停止")
                }
                lastCheckResult = false
            }
            result
        } catch (e: Exception) {
            // 网络不通，再用 pgrep 确认
            if (isProcessAlive() && isPortInUse(port)) {
                if (state != ServerState.RUNNING) {
                    state = ServerState.RUNNING
                    lastCheckResult = true
                    return true
                }
            }
            if (proc == null && state == ServerState.RUNNING) {
                state = ServerState.STOPPED
            }
            lastCheckResult = false
            false
        }
    }

    /**
     * 重置状态（服务被外部杀死时调用，如 START_STICKY 重建）
     */
    fun resetState() {
        process = null
        state = ServerState.STOPPED
        lastCheckResult = false
        lastCheckTime = 0
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

    fun discoverLanServers(): List<NetworkHelper.DiscoveredServer> {
        val results = mutableListOf<NetworkHelper.DiscoveredServer>()
        results.addAll(NetworkHelper.scanLanForServer(port, 200))
        results.addAll(NetworkHelper.discoverViaBroadcast(port))
        results.distinctBy { it.ip }.forEach {
            Log.i(TAG, "发现服务: ${it.ip}:${it.port} (${it.type})")
        }
        return results.distinctBy { it.ip }
    }

    fun findAvailableServer(): String? {
        if (isRunning()) {
            return getLocalUrl()
        }

        val servers = discoverLanServers()
        for (server in servers) {
            if (NetworkHelper.isReachable(server.ip, server.port, 500)) {
                return "http://${server.ip}:${server.port}"
            }
        }

        val config = ConfigManager.load()
        if (config.customUrl.isNotBlank()) {
            if (NetworkHelper.isReachable(config.customUrl.replace("http://", "").split(":")[0], port, 500)) {
                return config.customUrl
            }
        } else if (config.customIp.isNotBlank()) {
            if (NetworkHelper.isReachable(config.customIp, port, 500)) {
                return "http://${config.customIp}:$port"
            }
        }

        return null
    }

    fun executeCommand(command: String): Pair<Boolean, String> {
        return RootHelper.executeRoot(command)
    }

    fun executeRoot(command: String): Pair<Boolean, String> {
        return RootHelper.executeRoot(command)
    }
}
