package com.taskmod.app.tiles

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.util.Log
import android.widget.Toast
import com.taskmod.app.ServerManager
import com.taskmod.app.TaskModService
import kotlinx.coroutines.*

class UnlockTile : TileService() {

    companion object {
        private const val TAG = "UnlockTile"
    }

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onClick() {
        super.onClick()
        val service = TaskModService.getInstance()
        if (service == null) {
            android.os.Handler(mainLooper).post {
                Toast.makeText(this@UnlockTile, "服务未运行", Toast.LENGTH_SHORT).show()
            }
            Log.w(TAG, "服务未运行")
            return
        }

        scope.launch {
            try {
                val (success, _) = ServerManager.getInstance(this@UnlockTile).executeRoot(
                    "input keyevent KEYCODE_WAKEUP"
                )
                if (success) {
                    delay(300)
                    val (unlockSuccess, _) = ServerManager.getInstance(this@UnlockTile).executeRoot(
                        "input swipe 540 1800 540 600 300"
                    )
                    withContext(Dispatchers.Main) {
                        Toast.makeText(
                            this@UnlockTile,
                            if (unlockSuccess) "上滑解锁已执行" else "解锁失败",
                            Toast.LENGTH_SHORT
                        ).show()
                    }
                    Log.i(TAG, "解锁: ${if (unlockSuccess) "成功" else "失败"}")
                } else {
                    withContext(Dispatchers.Main) {
                        Toast.makeText(this@UnlockTile, "唤醒屏幕失败", Toast.LENGTH_SHORT).show()
                    }
                    Log.w(TAG, "唤醒屏幕失败")
                }
            } catch (e: Exception) {
                Log.e(TAG, "解锁失败", e)
                withContext(Dispatchers.Main) {
                    Toast.makeText(this@UnlockTile, "解锁失败: ${e.message}", Toast.LENGTH_SHORT).show()
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
}