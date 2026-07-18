package com.taskmod.app.tiles

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.util.Log
import android.widget.Toast
import com.taskmod.app.ServerManager
import com.taskmod.app.TaskModService
import kotlinx.coroutines.*

class ScreenshotTile : TileService() {

    companion object {
        private const val TAG = "ScreenshotTile"
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onClick() {
        super.onClick()
        val service = TaskModService.getInstance()
        if (service == null) {
            showToastWithHandler("服务未运行")
            Log.w(TAG, "服务未运行")
            return
        }

        scope.launch {
            try {
                val (success, result) = ServerManager.getInstance(this@ScreenshotTile).executeRoot(
                    "screencap -p /sdcard/Pictures/Screenshots/taskmod_${System.currentTimeMillis()}.png"
                )
                withContext(Dispatchers.Main) {
                    if (success) {
                        Toast.makeText(this@ScreenshotTile, "截屏成功", Toast.LENGTH_SHORT).show()
                    } else {
                        Toast.makeText(this@ScreenshotTile, "截屏失败: $result", Toast.LENGTH_SHORT).show()
                    }
                }
                Log.i(TAG, "截图: ${if (success) "成功" else "失败: $result"}")
            } catch (e: Exception) {
                Log.e(TAG, "截图失败", e)
                withContext(Dispatchers.Main) {
                    Toast.makeText(this@ScreenshotTile, "截图失败: ${e.message}", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    override fun onStartListening() {
        super.onStartListening()
        qsTile?.state = Tile.STATE_ACTIVE
        qsTile?.updateTile()
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }

    /**
     * TileService 中 showToast 需要特殊处理，因为 TileService 不是 Activity
     */
    private fun showToastWithHandler(message: String) {
        android.os.Handler(mainLooper).post {
            Toast.makeText(this@ScreenshotTile, message, Toast.LENGTH_SHORT).show()
        }
    }
}