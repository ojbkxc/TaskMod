package com.taskmod.app

import android.content.Context
import android.util.Log
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

            // 轮询等待服务就绪（最多 3 秒，每 200ms 检查一次）
            val maxWait = 3000L
            val interval = 200L
            var waited = 0L
            while (waited < maxWait) {
                Thread.sleep(interval)
                waited += interval
                if (isRunning()) break
            }

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

    @Volatile
    private var lastCheckTime: Long = 0
    @Volatile
    private var lastCheckResult: Boolean = false

    fun isRunning(): Boolean {
        val now = System.currentTimeMillis()
        if (now - lastCheckTime < 3000 && lastCheckResult && state == ServerState.RUNNING) {
            return true
        }

        val proc = process
        if (proc != null) {
            try {
                proc.exitValue()
                process = null
                state = ServerState.STOPPED
                lastCheckResult = false
                lastCheckTime = now
                return false
            } catch (e: IllegalThreadStateException) {
            }
        }

        lastCheckTime = now
        return try {
            val url = URL("http://127.0.0.1:$port/api/status")
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 500
            conn.readTimeout = 1000
            conn.requestMethod = "HEAD"
            conn.setRequestProperty("Connection", "close")
            val result = conn.responseCode == 200
            conn.disconnect()
            if (result && state != ServerState.RUNNING) {
                state = ServerState.RUNNING
            }
            lastCheckResult = result
            result
        } catch (e: Exception) {
            if (proc == null) {
                try {
                    val checkProcess = Runtime.getRuntime().exec(arrayOf("pgrep", "-f", BINARY_NAME))
                    val exitCode = checkProcess.waitFor()
                    if (exitCode == 0 && state != ServerState.RUNNING) {
                        state = ServerState.RUNNING
                        lastCheckResult = true
                        return true
                    }
                } catch (ex: Exception) {
                }
            }
            lastCheckResult = false
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
