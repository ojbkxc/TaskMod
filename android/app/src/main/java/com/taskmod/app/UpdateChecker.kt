package com.taskmod.app

import android.app.DownloadManager
import android.content.Context
import android.net.Uri
import android.os.Environment
import android.util.Log
import android.widget.Toast
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.gson.Gson
import com.google.gson.JsonObject
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.net.HttpURLConnection
import java.net.URL

class UpdateChecker(private val context: Context) {

    companion object {
        private const val TAG = "UpdateChecker"
        private const val API_URL = "https://api.github.com/repos/${TaskModApp.GITHUB_REPO}/releases/latest"
    }

    fun checkForUpdates(force: Boolean = false) {
        CoroutineScope(Dispatchers.IO).launch {
            try {
                val prefs = context.getSharedPreferences("taskmod", Context.MODE_PRIVATE)
                val lastCheck = prefs.getLong("last_update_check", 0)
                val now = System.currentTimeMillis()

                // 非强制检查时，每天只检查一次
                if (!force && now - lastCheck < 24 * 60 * 60 * 1000) {
                    return@launch
                }

                val url = URL(API_URL)
                val conn = url.openConnection() as HttpURLConnection
                conn.setRequestProperty("Accept", "application/vnd.github.v3+json")
                conn.connectTimeout = 5000
                conn.readTimeout = 5000

                if (conn.responseCode == 200) {
                    val response = conn.inputStream.bufferedReader().readText()
                    val json = Gson().fromJson(response, JsonObject::class.java)
                    val tagName = json.get("tag_name")?.asString ?: return@launch
                    val version = tagName.removePrefix("v")

                    // 获取当前版本
                    val currentVersion = try {
                        context.packageManager.getPackageInfo(context.packageName, 0).versionName
                    } catch (e: Exception) { "0.0.0" }

                    prefs.edit().putLong("last_update_check", now).apply()

                    if (isNewerVersion(version, currentVersion ?: "0.0.0")) {
                        val body = json.get("body")?.asString ?: ""
                        val assets = json.getAsJsonArray("assets")
                        var apkUrl = ""
                        for (asset in assets) {
                            val assetObj = asset.asJsonObject
                            val name = assetObj.get("name")?.asString ?: ""
                            if (name.endsWith(".apk")) {
                                apkUrl = assetObj.get("browser_download_url")?.asString ?: ""
                                break
                            }
                        }

                        withContext(Dispatchers.Main) {
                            showUpdateDialog(version, body, apkUrl)
                        }
                    } else if (force) {
                        withContext(Dispatchers.Main) {
                            Toast.makeText(context, "已是最新版本", Toast.LENGTH_SHORT).show()
                        }
                    }
                }
                conn.disconnect()
            } catch (e: Exception) {
                Log.e(TAG, "检查更新失败", e)
                if (force) {
                    withContext(Dispatchers.Main) {
                        Toast.makeText(context, "检查更新失败", Toast.LENGTH_SHORT).show()
                    }
                }
            }
        }
    }

    private fun isNewerVersion(newVersion: String, currentVersion: String): Boolean {
        val newParts = newVersion.split(".").map { it.toIntOrNull() ?: 0 }
        val currentParts = currentVersion.split(".").map { it.toIntOrNull() ?: 0 }
        for (i in 0 until maxOf(newParts.size, currentParts.size)) {
            val n = newParts.getOrElse(i) { 0 }
            val c = currentParts.getOrElse(i) { 0 }
            if (n > c) return true
            if (n < c) return false
        }
        return false
    }

    private fun showUpdateDialog(version: String, body: String, apkUrl: String) {
        MaterialAlertDialogBuilder(context)
            .setTitle("发现新版本 v$version")
            .setMessage(body.take(500))
            .setPositiveButton("下载更新") { _, _ ->
                if (apkUrl.isNotEmpty()) {
                    downloadUpdate(apkUrl)
                } else {
                    // 打开 GitHub Releases 页面
                    val intent = android.content.Intent(android.content.Intent.ACTION_VIEW, Uri.parse(
                        "https://github.com/${TaskModApp.GITHUB_REPO}/releases/tag/v$version"
                    ))
                    context.startActivity(intent)
                }
            }
            .setNegativeButton("跳过", null)
            .show()
    }

    private fun downloadUpdate(url: String) {
        try {
            val request = DownloadManager.Request(Uri.parse(url))
                .setTitle("TaskMod 更新")
                .setDescription("正在下载 TaskMod v${url}")
                .setDestinationInExternalFilesDir(context, null, "TaskMod-update.apk")
                .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)

            val dm = context.getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
            val downloadId = dm.enqueue(request)

            // 保存下载 ID
            context.getSharedPreferences("taskmod", Context.MODE_PRIVATE)
                .edit()
                .putLong("update_download_id", downloadId)
                .apply()

            Toast.makeText(context, "正在下载更新…", Toast.LENGTH_SHORT).show()
        } catch (e: Exception) {
            Log.e(TAG, "下载更新失败", e)
            Toast.makeText(context, "下载失败: ${e.message}", Toast.LENGTH_SHORT).show()
        }
    }
}
