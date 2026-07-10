package com.taskmod.app

import android.app.DownloadManager
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.util.Log
import androidx.core.content.FileProvider
import java.io.File

class UpdateReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "UpdateReceiver"
    }

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == DownloadManager.ACTION_DOWNLOAD_COMPLETE) {
            val downloadId = intent.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1)
            val prefs = context.getSharedPreferences("taskmod", Context.MODE_PRIVATE)
            val savedDownloadId = prefs.getLong("update_download_id", -1)

            if (downloadId == savedDownloadId) {
                Log.i(TAG, "更新下载完成")
                installUpdate(context)
            }
        }
    }

    private fun installUpdate(context: Context) {
        val file = File(context.getExternalFilesDir(null), "TaskMod-update.apk")
        if (!file.exists()) {
            Log.e(TAG, "更新文件不存在")
            return
        }

        val uri = FileProvider.getUriForFile(
            context,
            "${context.packageName}.fileprovider",
            file
        )

        val intent = Intent(Intent.ACTION_VIEW).apply {
            setDataAndType(uri, "application/vnd.android.package-archive")
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)
        }

        context.startActivity(intent)
    }
}
