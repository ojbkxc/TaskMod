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
        private const val AUTO_RESTART_DELAY = 3000L
        private const val MAX_AUTO_RESTARTS = 5
        private const val AUTO_RESTART_WINDOW = 60000L

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
    
    @Volatile
    private var autoRestartCount = 0
    @Volatile
    private var lastAutoRestartTime = 0L
    @Volatile
    private var autoRestartScheduled = false

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

            val maxWait = 8000L
            val interval = 200L
            var waited = 0L
            while (waited < maxWait) {
                Thread.sleep(interval)
                waited += interval
                if (isPortInUse(port)) break
            }

            if (isPortInUse(port)) {
                state = ServerState.RUNNING
                autoRestartCount = 0
                lastAutoRestartTime = 0L
                Log.i(TAG, "服务已启动，端口: $port")
                startProcessMonitor()
                return true
            } else {
                var outputLog = ""
                try {
                    process?.inputStream?.bufferedReader()?.use { reader ->
                        outputLog = reader.readText().take(500)
                    }
                } catch (_: Exception) {}
                lastError = "进程启动后立即退出${if (outputLog.isNotBlank()) ": $outputLog" else ""}"
                state = ServerState.ERROR
                process = null
                scheduleAutoRestart()
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
    @Volatile
    private var isChecking = false

    fun isRunning(): Boolean {
        val now = System.currentTimeMillis()
        
        if (state == ServerState.STARTING) {
            return isPortInUse(port)
        }

        if (now - lastCheckTime < 2000 && lastCheckResult && state == ServerState.RUNNING) {
            return true
        }

        if (state == ServerState.STOPPED && !lastCheckResult && now - lastCheckTime < 1000) {
            return false
        }

        val proc = process
        if (proc != null) {
            try {
                proc.exitValue()
                process = null
            } catch (_: IllegalThreadStateException) {
            }
        }

        lastCheckTime = now

        return try {
            val url = URL("http://127.0.0.1:$port/api/status")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 300
            conn.readTimeout = 500
            conn.requestMethod = "HEAD"
            conn.setRequestProperty("Connection", "close")
            val result = conn.responseCode == 200
            conn.disconnect()

            updateStateBasedOnResult(result, proc)
            result
        } catch (_: Exception) {
            val fallback = isProcessAlive() && isPortInUse(port)
            updateStateBasedOnResult(fallback, proc)
            fallback
        }
    }

    private fun updateStateBasedOnResult(result: Boolean, proc: Process?) {
        if (result) {
            if (state != ServerState.RUNNING) {
                state = ServerState.RUNNING
                Log.i(TAG, "检测到服务已在运行")
            }
            lastCheckResult = true
        } else {
            if (proc == null && state == ServerState.RUNNING) {
                state = ServerState.STOPPED
                Log.i(TAG, "检测到服务已停止")
            }
            lastCheckResult = false
        }
    }

    fun isRunningFast(): Boolean {
        if (state == ServerState.STARTING) {
            return false
        }
        if (state == ServerState.RUNNING && lastCheckResult) {
            val now = System.currentTimeMillis()
            if (now - lastCheckTime < 5000) {
                return true
            }
        }
        return isRunning()
    }

    private fun startProcessMonitor() {
        Thread {
            while (true) {
                Thread.sleep(2000)
                val proc = process
                if (proc == null) break
                
                try {
                    proc.exitValue()
                    Log.w(TAG, "检测到进程意外退出，尝试自动重启")
                    process = null
                    scheduleAutoRestart()
                    break
                } catch (_: IllegalThreadStateException) {
                }
            }
        }.start()
    }

    private fun scheduleAutoRestart() {
        if (autoRestartScheduled) return
        
        val now = System.currentTimeMillis()
        if (now - lastAutoRestartTime > AUTO_RESTART_WINDOW) {
            autoRestartCount = 0
        }
        
        if (autoRestartCount >= MAX_AUTO_RESTARTS) {
            Log.w(TAG, "已达最大自动重启次数($MAX_AUTO_RESTARTS)，停止自动重启")
            return
        }
        
        autoRestartScheduled = true
        Thread {
            Thread.sleep(AUTO_RESTART_DELAY)
            autoRestartScheduled = false
            
            if (state == ServerState.STOPPED) return@Thread
            
            autoRestartCount++
            lastAutoRestartTime = System.currentTimeMillis()
            Log.i(TAG, "自动重启服务 ($autoRestartCount/$MAX_AUTO_RESTARTS)")
            
            start()
        }.start()
    }

    fun resetState() {
        process = null
        state = ServerState.STOPPED
        lastCheckResult = false
        lastCheckTime = 0
        autoRestartCount = 0
        autoRestartScheduled = false
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
