package com.taskmod.app

import android.util.Log
import com.google.gson.Gson
import com.google.gson.GsonBuilder
import java.io.File

/**
 * 统一配置管理 - 与 Magisk 模块共享同一份配置
 * 存储路径: /sdcard/TaskMod/app_settings.json
 */
object ConfigManager {

    private const val TAG = "ConfigManager"
    private const val TASKMOD_DIR = "/sdcard/TaskMod"
    private const val CONFIG_FILE = "$TASKMOD_DIR/app_settings.json"

    data class AppConfig(
        val port: Int = 9527,
        val customUrl: String = "",
        val autoStart: Boolean = true,
        val customIp: String = ""
    )

    private var cached: AppConfig? = null
    private val gson: Gson = GsonBuilder().setPrettyPrinting().create()

    /** 确保目录存在 */
    fun ensureDir(): Boolean {
        val dir = File(TASKMOD_DIR)
        if (!dir.exists()) {
            val ok = dir.mkdirs()
            if (!ok) {
                Log.w(TAG, "无法创建 $TASKMOD_DIR，尝试用 su")
                try {
                    Runtime.getRuntime().exec(arrayOf("su", "-c", "mkdir -p $TASKMOD_DIR")).waitFor()
                    Runtime.getRuntime().exec(arrayOf("su", "-c", "chmod 777 $TASKMOD_DIR")).waitFor()
                } catch (e: Exception) {
                    Log.e(TAG, "创建目录失败", e)
                    return false
                }
            }
        }
        return true
    }

    /** 加载配置 */
    fun load(): AppConfig {
        cached?.let { return it }
        val file = File(CONFIG_FILE)
        val config = if (file.exists()) {
            try {
                val json = file.readText()
                gson.fromJson(json, AppConfig::class.java) ?: AppConfig()
            } catch (e: Exception) {
                Log.w(TAG, "读取配置失败，使用默认值", e)
                AppConfig()
            }
        } else {
            AppConfig()
        }
        cached = config
        return config
    }

    /** 保存配置 */
    fun save(config: AppConfig) {
        ensureDir()
        try {
            File(CONFIG_FILE).writeText(gson.toJson(config))
            cached = config
            Log.i(TAG, "配置已保存: $CONFIG_FILE")
        } catch (e: Exception) {
            Log.e(TAG, "保存配置失败", e)
        }
    }

    /** 更新单个字段 */
    fun update(block: AppConfig.() -> AppConfig) {
        save(load().block())
    }

    /** 获取端口 */
    fun getPort(): Int = load().port

    /** 获取自定义 URL（优先级: customUrl > customIp+port > 自动检测） */
    fun getAccessUrl(): String {
        val config = load()
        if (config.customUrl.isNotBlank()) return config.customUrl
        if (config.customIp.isNotBlank()) return "http://${config.customIp}:${config.port}"
        return "http://${NetworkHelper.getLocalIpAddress()}:${config.port}"
    }

    /** 清除缓存（外部修改文件后调用） */
    fun invalidate() { cached = null }
}
