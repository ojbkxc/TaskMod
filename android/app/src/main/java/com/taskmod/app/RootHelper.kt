package com.taskmod.app

import java.io.File

object RootHelper {

    data class RootResult(
        val hasRoot: Boolean,
        val method: String // "magisk", "su", "none"
    )

    fun checkRoot(): RootResult {
        // 检查 su 二进制
        val suPaths = listOf(
            "/system/bin/su", "/system/xbin/su",
            "/sbin/su", "/data/local/su",
            "/data/local/bin/su", "/data/local/xbin/su"
        )
        val hasSu = suPaths.any { File(it).exists() } || try {
            val process = Runtime.getRuntime().exec(arrayOf("which", "su"))
            process.waitFor() == 0
        } catch (e: Exception) { false }

        if (!hasSu) return RootResult(false, "none")

        // 检查 Magisk
        val hasMagisk = File("/data/adb/magisk").exists() ||
                File("/sbin/magisk").exists() ||
                try {
                    val process = Runtime.getRuntime().exec(arrayOf("su", "-c", "magisk -v"))
                    process.waitFor() == 0
                } catch (e: Exception) { false }

        return if (hasMagisk) RootResult(true, "magisk") else RootResult(true, "su")
    }

    fun executeRoot(command: String): Pair<Boolean, String> {
        return try {
            val process = Runtime.getRuntime().exec(arrayOf("su", "-c", command))
            val output = process.inputStream.bufferedReader().readText()
            val error = process.errorStream.bufferedReader().readText()
            val exitCode = process.waitFor()
            if (exitCode == 0) Pair(true, output.trim()) else Pair(false, error.trim())
        } catch (e: Exception) {
            Pair(false, e.message ?: "执行失败")
        }
    }

    fun isMagiskModuleInstalled(): Boolean {
        return File("/data/adb/modules/TaskMod").exists()
    }
}
