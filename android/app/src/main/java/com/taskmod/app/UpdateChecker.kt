package com.taskmod.app

import android.app.AlertDialog
import android.app.DownloadManager
import android.content.Context
import android.net.Uri
import android.os.Environment
import android.util.Log
import android.widget.Toast
import kotlinx.coroutines.*
import org.json.JSONObject
import java.net.HttpURLConnection
import java.net.URL

data class UpdateInfo(
    val version: String,
    val versionCode: Int,
    val downloadUrl: String,
    val changelog: String
)

object UpdateChecker {

    private const val TAG = "UpdateChecker"
    private const val GITHUB_API = "https://api.github.com/repos/mazy16/TaskMod/releases/latest"
    private const val GITHUB_RELEASES = "https://github.com/mazy16/TaskMod/releases"

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var currentJob: Job? = null

    fun checkUpdate(
        context: Context,
        onResult: (UpdateInfo?) -> Unit
    ) {
        // 取消之前的检查任务
        currentJob?.cancel()
        currentJob = scope.launch {
            try {
                val info = checkUpdateInternal(context)
                withContext(Dispatchers.Main) {
                    onResult(info)
                }
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "检查更新失败", e)
                withContext(Dispatchers.Main) {
                    onResult(null)
                }
            }
        }
    }

    private fun checkUpdateInternal(context: Context): UpdateInfo? {
        val currentVersion = getCurrentVersion(context)
        val currentVersionCode = getCurrentVersionCode(context)

        val latestRelease = fetchLatestRelease()
        if (latestRelease == null) {
            Log.w(TAG, "无法获取最新版本信息")
            return null
        }

        val latestVersion = latestRelease["version"] ?: return null
        val latestVersionCode = latestRelease["versionCode"]?.toIntOrNull() ?: return null
        val downloadUrl = latestRelease["downloadUrl"] ?: ""
        val changelog = latestRelease["changelog"] ?: ""

        Log.i(TAG, "当前版本: $currentVersion ($currentVersionCode), 最新版本: $latestVersion ($latestVersionCode)")

        if (latestVersionCode > currentVersionCode) {
            return UpdateInfo(
                version = latestVersion,
                versionCode = latestVersionCode,
                downloadUrl = downloadUrl,
                changelog = changelog
            )
        }
        return null
    }

    private fun fetchLatestRelease(): Map<String, String>? {
        return try {
            val url = URL(GITHUB_API)
            val conn = url.openConnection() as HttpURLConnection
            conn.connectTimeout = 10000
            conn.readTimeout = 10000
            conn.setRequestProperty("Accept", "application/vnd.github.v3+json")
            conn.setRequestProperty("User-Agent", "TaskMod-UpdateChecker")

            if (conn.responseCode != 200) {
                Log.w(TAG, "GitHub API 返回: ${conn.responseCode}")
                return null
            }

            val jsonText = conn.inputStream.bufferedReader().use { it.readText() }
            conn.disconnect()

            val json = JSONObject(jsonText)
            val tagName = json.optString("tag_name", "")
            val version = tagName.removePrefix("v")
            val body = json.optString("body", "")

            // 从 release body 中提取 versionCode
            val versionCodeMatch = Regex("versionCode[：:]\\s*(\\d+)").find(body)
            val versionCode = versionCodeMatch?.groupValues?.get(1) ?: "0"

            // 查找 APK 下载链接
            var downloadUrl = ""
            val assets = json.optJSONArray("assets")
            if (assets != null) {
                for (i in 0 until assets.length()) {
                    val asset = assets.getJSONObject(i)
                    val name = asset.optString("name", "")
                    if (name.endsWith(".apk")) {
                        downloadUrl = asset.optString("browser_download_url", "")
                        break
                    }
                }
            }

            mapOf(
                "version" to version,
                "versionCode" to versionCode,
                "downloadUrl" to downloadUrl,
                "changelog" to body
            )
        } catch (e: Exception) {
            Log.e(TAG, "获取 Release 信息失败", e)
            null
        }
    }

    fun downloadUpdate(context: Context, updateInfo: UpdateInfo) {
        scope.launch {
            try {
                val downloadManager = context.getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
                val uri = Uri.parse(updateInfo.downloadUrl)
                val request = DownloadManager.Request(uri).apply {
                    setTitle("TaskMod 更新")
                    setDescription("正在下载 TaskMod ${updateInfo.version}")
                    setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
                    setDestinationInExternalFilesDir(context, Environment.DIRECTORY_DOWNLOADS, "TaskMod-update.apk")
                    setAllowedOverMetered(true)
                    setAllowedOverRoaming(true)
                }

                val downloadId = downloadManager.enqueue(request)
                context.getSharedPreferences("taskmod", Context.MODE_PRIVATE)
                    .edit()
                    .putLong("update_download_id", downloadId)
                    .apply()
                Log.i(TAG, "开始下载更新: ${updateInfo.version}, id=$downloadId")
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                Log.e(TAG, "下载更新失败", e)
            }
        }
    }

    fun getCurrentVersion(context: Context): String {
        return try {
            val packageInfo = context.packageManager.getPackageInfo(context.packageName, 0)
            packageInfo.versionName ?: "1.0.0"
        } catch (e: Exception) {
            "1.0.0"
        }
    }

    private fun getCurrentVersionCode(context: Context): Int {
        return try {
            val packageInfo = context.packageManager.getPackageInfo(context.packageName, 0)
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.P) {
                packageInfo.longVersionCode.toInt()
            } else {
                @Suppress("DEPRECATION")
                packageInfo.versionCode
            }
        } catch (e: Exception) {
            1
        }
    }

    fun getReleasesUrl(): String = GITHUB_RELEASES

    /**
     * 检查更新（静默模式，有更新时弹窗提示）
     */
    fun checkForUpdates(context: Context, force: Boolean = false) {
        checkUpdate(context) { info ->
            if (info != null) {
                showUpdateDialog(context, info)
            } else if (force) {
                // 强制检查时，没有更新也提示
                Toast.makeText(context, "已是最新版本", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun showUpdateDialog(context: Context, info: UpdateInfo) {
        AlertDialog.Builder(context)
            .setTitle("发现新版本")
            .setMessage("版本: ${info.version}\n\n更新内容:\n${info.changelog}")
            .setPositiveButton("下载更新") { _, _ -> downloadUpdate(context, info) }
            .setNegativeButton("稍后", null)
            .show()
    }
}