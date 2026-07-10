package com.taskmod.app

import android.content.Context
import android.util.Log
import org.json.JSONArray
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

/**
 * cloudflared 隧道守护进程管理器
 *
 * 支持多隧道、多服务的增删改查和独立控制
 */
class DaemonManager private constructor(private val context: Context) {

    companion object {
        private const val TAG = "DaemonManager"

        @Volatile
        private var instance: DaemonManager? = null

        fun getInstance(context: Context): DaemonManager {
            return instance ?: synchronized(this) {
                instance ?: DaemonManager(context.applicationContext).also { instance = it }
            }
        }
    }

    /** 隧道信息 */
    data class TunnelInfo(
        val name: String,
        val token: String,
        val enabled: Boolean,
        val services: List<ServiceInfo>
    )

    /** 服务信息 */
    data class ServiceInfo(
        val name: String,
        val url: String,
        val enabled: Boolean
    )

    /** 进程状态 */
    data class ProcessStatus(
        val tunnelName: String,
        val pid: Int,
        val uptimeSeconds: Long,
        val isAlive: Boolean
    )

    /** 守护进程整体状态 */
    data class DaemonStatus(
        val pid: Int,
        val uptimeSeconds: Long,
        val tunnelCount: Int,
        val activeTunnelCount: Int
    )

    /** API 响应 */
    data class ApiResponse<T>(
        val success: Boolean,
        val data: T?,
        val error: String?
    )

    // ========== 守护进程控制 ==========

    /** 获取守护进程整体状态 */
    fun getStatus(): ApiResponse<DaemonStatus> {
        return try {
            val response = makeRequest("GET", "/api/daemon/status")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                val data = json.optJSONObject("data")
                if (data != null) {
                    ApiResponse(true, DaemonStatus(
                        pid = data.optInt("pid", 0),
                        uptimeSeconds = data.optLong("uptime_secs", 0),
                        tunnelCount = data.optInt("tunnel_count", 0),
                        activeTunnelCount = data.optInt("active_tunnel_count", 0)
                    ), null)
                } else {
                    ApiResponse(true, null, null)
                }
            } else {
                ApiResponse(false, null, json.optString("error", "未知错误"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "获取守护进程状态失败", e)
            ApiResponse(false, null, "获取失败: ${e.message}")
        }
    }

    /** 启动守护进程（启用所有 enabled 隧道） */
    fun start(): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/daemon/restart")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已启动"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "启动失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "启动守护进程失败", e)
            ApiResponse(false, null, "启动失败: ${e.message}")
        }
    }

    /** 停止守护进程（停止所有隧道） */
    fun stop(): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/daemon/stop")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已停止"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "停止失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "停止守护进程失败", e)
            ApiResponse(false, null, "停止失败: ${e.message}")
        }
    }

    /** 重启守护进程（热重载所有隧道） */
    fun restart(): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/daemon/restart")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已重启"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "重启失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "重启守护进程失败", e)
            ApiResponse(false, null, "重启失败: ${e.message}")
        }
    }

    // ========== 隧道管理 ==========

    /** 获取所有隧道 */
    fun listTunnels(): ApiResponse<List<TunnelInfo>> {
        return try {
            val response = makeRequest("GET", "/api/tunnels")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                val dataArray = json.optJSONArray("data")
                val tunnels = mutableListOf<TunnelInfo>()

                if (dataArray != null) {
                    for (i in 0 until dataArray.length()) {
                        val tunnelJson = dataArray.getJSONObject(i)
                        tunnels.add(parseTunnelInfo(tunnelJson))
                    }
                }

                ApiResponse(true, tunnels, null)
            } else {
                ApiResponse(false, null, json.optString("error", "未知错误"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "获取隧道列表失败", e)
            ApiResponse(false, null, "获取失败: ${e.message}")
        }
    }

    /** 获取隧道详情 */
    fun getTunnel(name: String): ApiResponse<TunnelInfo?> {
        return try {
            val response = makeRequest("GET", "/api/tunnels/$name")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                val data = json.optJSONObject("data")
                if (data != null) {
                    ApiResponse(true, parseTunnelInfo(data), null)
                } else {
                    ApiResponse(true, null, null)
                }
            } else {
                ApiResponse(false, null, json.optString("error", "未知错误"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "获取隧道详情失败", e)
            ApiResponse(false, null, "获取失败: ${e.message}")
        }
    }

    /** 添加隧道 */
    fun addTunnel(name: String, token: String, enabled: Boolean = true): ApiResponse<String> {
        return try {
            val body = JSONObject().apply {
                put("name", name)
                put("token", token)
                put("enabled", enabled)
            }
            val response = makeRequest("POST", "/api/tunnels", body.toString())
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已添加"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "添加失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "添加隧道失败", e)
            ApiResponse(false, null, "添加失败: ${e.message}")
        }
    }

    /** 更新隧道 */
    fun updateTunnel(name: String, newName: String? = null, token: String? = null, enabled: Boolean? = null): ApiResponse<String> {
        return try {
            val body = JSONObject().apply {
                newName?.let { put("new_name", it) }
                token?.let { put("token", it) }
                enabled?.let { put("enabled", it) }
            }
            val response = makeRequest("PUT", "/api/tunnels/$name", body.toString())
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已更新"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "更新失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "更新隧道失败", e)
            ApiResponse(false, null, "更新失败: ${e.message}")
        }
    }

    /** 删除隧道 */
    fun deleteTunnel(name: String): ApiResponse<String> {
        return try {
            val response = makeRequest("DELETE", "/api/tunnels/$name")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已删除"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "删除失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "删除隧道失败", e)
            ApiResponse(false, null, "删除失败: ${e.message}")
        }
    }

    /** 启用隧道 */
    fun enableTunnel(name: String): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/tunnels/$name/enable")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已启用"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "启用失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "启用隧道失败", e)
            ApiResponse(false, null, "启用失败: ${e.message}")
        }
    }

    /** 禁用隧道 */
    fun disableTunnel(name: String): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/tunnels/$name/disable")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已禁用"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "禁用失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "禁用隧道失败", e)
            ApiResponse(false, null, "禁用失败: ${e.message}")
        }
    }

    /** 启动隧道进程 */
    fun startTunnel(name: String): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/tunnels/$name/start")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已启动"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "启动失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "启动隧道失败", e)
            ApiResponse(false, null, "启动失败: ${e.message}")
        }
    }

    /** 停止隧道进程 */
    fun stopTunnel(name: String): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/tunnels/$name/stop")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已停止"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "停止失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "停止隧道失败", e)
            ApiResponse(false, null, "停止失败: ${e.message}")
        }
    }

    /** 重启隧道 */
    fun restartTunnel(name: String): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/tunnels/$name/restart")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已重启"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "重启失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "重启隧道失败", e)
            ApiResponse(false, null, "重启失败: ${e.message}")
        }
    }

    // ========== 服务管理 ==========

    /** 获取隧道下的服务列表 */
    fun listServices(tunnelName: String): ApiResponse<List<ServiceInfo>> {
        return try {
            val response = makeRequest("GET", "/api/tunnels/$tunnelName/services")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                val dataArray = json.optJSONArray("data")
                val services = mutableListOf<ServiceInfo>()

                if (dataArray != null) {
                    for (i in 0 until dataArray.length()) {
                        val serviceJson = dataArray.getJSONObject(i)
                        services.add(parseServiceInfo(serviceJson))
                    }
                }

                ApiResponse(true, services, null)
            } else {
                ApiResponse(false, null, json.optString("error", "未知错误"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "获取服务列表失败", e)
            ApiResponse(false, null, "获取失败: ${e.message}")
        }
    }

    /** 添加服务 */
    fun addService(tunnelName: String, serviceName: String, url: String, enabled: Boolean = true): ApiResponse<String> {
        return try {
            val body = JSONObject().apply {
                put("name", serviceName)
                put("url", url)
                put("enabled", enabled)
            }
            val response = makeRequest("POST", "/api/tunnels/$tunnelName/services", body.toString())
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已添加"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "添加失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "添加服务失败", e)
            ApiResponse(false, null, "添加失败: ${e.message}")
        }
    }

    /** 更新服务 */
    fun updateService(tunnelName: String, serviceName: String, newName: String? = null, url: String? = null, enabled: Boolean? = null): ApiResponse<String> {
        return try {
            val body = JSONObject().apply {
                newName?.let { put("new_name", it) }
                url?.let { put("url", it) }
                enabled?.let { put("enabled", it) }
            }
            val response = makeRequest("PUT", "/api/tunnels/$tunnelName/services/$serviceName", body.toString())
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已更新"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "更新失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "更新服务失败", e)
            ApiResponse(false, null, "更新失败: ${e.message}")
        }
    }

    /** 删除服务 */
    fun deleteService(tunnelName: String, serviceName: String): ApiResponse<String> {
        return try {
            val response = makeRequest("DELETE", "/api/tunnels/$tunnelName/services/$serviceName")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已删除"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "删除失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "删除服务失败", e)
            ApiResponse(false, null, "删除失败: ${e.message}")
        }
    }

    /** 启用服务 */
    fun enableService(tunnelName: String, serviceName: String): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/tunnels/$tunnelName/services/$serviceName/enable")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已启用"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "启用失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "启用服务失败", e)
            ApiResponse(false, null, "启用失败: ${e.message}")
        }
    }

    /** 禁用服务 */
    fun disableService(tunnelName: String, serviceName: String): ApiResponse<String> {
        return try {
            val response = makeRequest("POST", "/api/tunnels/$tunnelName/services/$serviceName/disable")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                ApiResponse(true, json.optString("data", "已禁用"), null)
            } else {
                ApiResponse(false, null, json.optString("error", "禁用失败"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "禁用服务失败", e)
            ApiResponse(false, null, "禁用失败: ${e.message}")
        }
    }

    // ========== 进程状态 ==========

    /** 获取所有进程状态 */
    fun listProcesses(): ApiResponse<List<ProcessStatus>> {
        return try {
            val response = makeRequest("GET", "/api/processes")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                val dataArray = json.optJSONArray("data")
                val processes = mutableListOf<ProcessStatus>()

                if (dataArray != null) {
                    for (i in 0 until dataArray.length()) {
                        val processJson = dataArray.getJSONObject(i)
                        processes.add(parseProcessStatus(processJson))
                    }
                }

                ApiResponse(true, processes, null)
            } else {
                ApiResponse(false, null, json.optString("error", "未知错误"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "获取进程状态失败", e)
            ApiResponse(false, null, "获取失败: ${e.message}")
        }
    }

    /** 获取指定隧道的进程状态 */
    fun getProcessStatus(tunnelName: String): ApiResponse<ProcessStatus?> {
        return try {
            val response = makeRequest("GET", "/api/processes/$tunnelName")
            val json = JSONObject(response)

            if (json.optBoolean("success", false)) {
                val data = json.optJSONObject("data")
                if (data != null) {
                    ApiResponse(true, parseProcessStatus(data), null)
                } else {
                    ApiResponse(true, null, null)
                }
            } else {
                ApiResponse(false, null, json.optString("error", "未知错误"))
            }
        } catch (e: Exception) {
            Log.e(TAG, "获取进程状态失败", e)
            ApiResponse(false, null, "获取失败: ${e.message}")
        }
    }

    // ========== 工具方法 ==========

    /** 解析隧道信息 */
    private fun parseTunnelInfo(json: JSONObject): TunnelInfo {
        val servicesArray = json.optJSONArray("services")
        val services = mutableListOf<ServiceInfo>()

        if (servicesArray != null) {
            for (i in 0 until servicesArray.length()) {
                services.add(parseServiceInfo(servicesArray.getJSONObject(i)))
            }
        }

        return TunnelInfo(
            name = json.optString("name", ""),
            token = json.optString("token", ""),
            enabled = json.optBoolean("enabled", true),
            services = services
        )
    }

    /** 解析服务信息 */
    private fun parseServiceInfo(json: JSONObject): ServiceInfo {
        return ServiceInfo(
            name = json.optString("name", ""),
            url = json.optString("url", ""),
            enabled = json.optBoolean("enabled", true)
        )
    }

    /** 解析进程状态 */
    private fun parseProcessStatus(json: JSONObject): ProcessStatus {
        return ProcessStatus(
            tunnelName = json.optString("tunnel_name", ""),
            pid = json.optInt("pid", 0),
            uptimeSeconds = json.optLong("uptime_secs", 0),
            isAlive = json.optBoolean("is_alive", false)
        )
    }

    /** 格式化运行时长 */
    fun formatUptime(seconds: Long): String {
        return when {
            seconds < 60 -> "$seconds 秒"
            seconds < 3600 -> "${seconds / 60} 分 ${seconds % 60} 秒"
            seconds < 86400 -> "${seconds / 3600} 时 ${(seconds % 3600) / 60} 分"
            else -> "${seconds / 86400} 天 ${(seconds % 86400) / 3600} 时"
        }
    }

    /** 发送 HTTP 请求 */
    private fun makeRequest(method: String, path: String, body: String? = null): String {
        val serverManager = ServerManager.getInstance(context)
        val port = serverManager.port
        val url = URL("http://127.0.0.1:$port$path")

        val conn = url.openConnection() as HttpURLConnection
        conn.requestMethod = method
        conn.connectTimeout = 5000
        conn.readTimeout = 5000

        if (body != null) {
            conn.doOutput = true
            conn.setRequestProperty("Content-Type", "application/json")
            conn.outputStream.bufferedWriter().use { it.write(body) }
        }

        val response = conn.inputStream.bufferedReader().use { it.readText() }
        conn.disconnect()

        return response
    }
}