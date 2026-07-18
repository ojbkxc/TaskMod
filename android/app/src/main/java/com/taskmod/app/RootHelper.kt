package com.taskmod.app

import android.util.Log
import java.io.BufferedReader
import java.io.DataOutputStream
import java.io.File
import java.io.InputStreamReader
import java.util.concurrent.*

/**
 * Root 权限管理 - 使用持久化 su shell 提升性能
 * 参考 agent-toolbox 的 RootManager 设计，维持单一 su 进程避免重复创建开销
 */
object RootHelper {

    private const val TAG = "RootHelper"

    @Volatile
    private var hasRootAccess: Boolean? = null

    private var suProcess: Process? = null
    private var suOutputStream: DataOutputStream? = null
    private var suReader: BufferedReader? = null
    private val suLock = Any()

    /** 复用线程池，避免每次命令执行创建新线程池 */
    private val commandExecutor: ExecutorService = Executors.newCachedThreadPool { r ->
        Thread(r, "RootHelper-Command").apply { isDaemon = true }
    }

    data class RootResult(
        val hasRoot: Boolean,
        val method: String // "magisk", "su", "none"
    )

    // ========== Root 检测 ==========

    fun checkRoot(): RootResult {
        synchronized(suLock) {
            if (hasRootAccess == null) {
                initSuShell()
            }
        }
        val hasRoot = hasRootAccess == true
        if (!hasRoot) return RootResult(false, "none")

        val hasMagisk = File("/data/adb/magisk").exists() ||
                File("/sbin/magisk").exists() ||
                try {
                    val (success, result) = executeRoot("magisk -v")
                    success && result.isNotEmpty()
                } catch (e: Exception) {
                    false
                }

        return if (hasMagisk) RootResult(true, "magisk") else RootResult(true, "su")
    }

    fun isMagiskModuleInstalled(): Boolean {
        return File("/data/adb/modules/TaskMod").exists()
    }

    // ========== 持久化 su shell ==========

    /**
     * 初始化 su shell，带超时保护防止永久阻塞
     */
    private fun initSuShell(): Boolean {
        synchronized(suLock) {
            if (hasRootAccess == true) return true

            try {
                suProcess = Runtime.getRuntime().exec("su")
                suOutputStream = DataOutputStream(suProcess!!.outputStream)
                suReader = BufferedReader(InputStreamReader(suProcess!!.inputStream))

                // 带超时的 root 检测（5 秒）
                val result = executeCommandInternalWithTimeout("id", 5000L)
                hasRootAccess = result != null && result.contains("uid=0")

                if (hasRootAccess != true) {
                    closeSuShell()
                }
                Log.i(TAG, "su shell 初始化: hasRoot=$hasRootAccess")
                return hasRootAccess == true
            } catch (e: Exception) {
                Log.e(TAG, "su shell 初始化失败", e)
                hasRootAccess = false
                closeSuShell()
                return false
            }
        }
    }

    /**
     * 执行 root 命令（公开接口）
     * @return Pair<success, output>
     */
    fun executeRoot(command: String): Pair<Boolean, String> {
        synchronized(suLock) {
            if (hasRootAccess != true) {
                if (!initSuShell()) {
                    return Pair(false, "无法获取 Root 权限")
                }
            }
            val result = executeCommandInternal(command)
            if (result == null) {
                // su shell 可能断开，尝试重新初始化
                closeSuShell()
                if (!initSuShell()) {
                    return Pair(false, "su shell 断开且无法重连")
                }
                val retry = executeCommandInternal(command)
                return if (retry != null) Pair(true, retry) else Pair(false, "命令执行失败")
            }
            return Pair(true, result)
        }
    }

    /**
     * 内部命令执行 - 使用 marker 机制检测命令完成
     * 带 30 秒超时保护，防止因命令无输出导致永久阻塞
     */
    private fun executeCommandInternal(command: String): String? {
        return executeCommandInternalWithTimeout(command, 30000L)
    }

    private fun executeCommandInternalWithTimeout(command: String, timeoutMs: Long): String? {
        val os = suOutputStream
        val reader = suReader
        if (os == null || reader == null) return null

        val future = commandExecutor.submit<String?> {
            try {
                val marker = "CMD_DONE_${System.nanoTime()}"

                os.writeBytes("$command\n")
                os.writeBytes("echo $marker\n")
                os.flush()

                val output = StringBuilder()
                var line: String?
                while (reader.readLine().also { line = it } != null) {
                    if (line!!.contains(marker)) break
                    output.append(line).append("\n")
                }

                output.toString().trim()
            } catch (e: Exception) {
                Log.e(TAG, "命令执行异常: $command", e)
                null
            }
        }

        return try {
            future.get(timeoutMs, TimeUnit.MILLISECONDS)
        } catch (e: TimeoutException) {
            Log.e(TAG, "命令执行超时 (${timeoutMs}ms): $command", e)
            future.cancel(true)
            // 超时后 su shell 可能处于不稳定状态，标记为断开
            closeSuShell()
            null
        } catch (e: Exception) {
            Log.e(TAG, "命令执行失败: $command", e)
            if (e is InterruptedException) {
                Thread.currentThread().interrupt()
            }
            null
        }
    }

    private fun closeSuShell() {
        synchronized(suLock) {
            try {
                suOutputStream?.let {
                    it.writeBytes("exit\n")
                    it.flush()
                    it.close()
                }
            } catch (_: Exception) {}
            try { suReader?.close() } catch (_: Exception) {}
            try { suProcess?.destroy() } catch (_: Exception) {}
            suProcess = null
            suOutputStream = null
            suReader = null
            hasRootAccess = null
        }
    }

    /**
     * 重置 Root 状态（用于外部权限变化后重新检测）
     */
    fun resetRootStatus() {
        closeSuShell()
    }

    /**
     * 获取 Root 状态描述
     */
    fun getRootStatus(): String {
        return when (hasRootAccess) {
            true -> "已获取 Root 权限"
            false -> "未获取 Root 权限"
            null -> "未检测"
        }
    }
}