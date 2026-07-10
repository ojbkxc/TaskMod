package com.taskmod.app.tiles

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.widget.Toast
import com.taskmod.app.RootHelper
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch

class UnlockTile : TileService() {

    override fun onStartListening() {
        super.onStartListening()
        qsTile?.let {
            it.state = Tile.STATE_ACTIVE
            it.updateTile()
        }
    }

    override fun onClick() {
        super.onClick()
        CoroutineScope(Dispatchers.IO).launch {
            try {
                RootHelper.executeRoot("input keyevent KEYCODE_WAKEUP")
                delay(300)
                val (success, _) = RootHelper.executeRoot("input swipe 540 1800 540 600 300")
                android.os.Handler(mainLooper).post {
                    Toast.makeText(this@UnlockTile, if (success) "上滑解锁已执行" else "解锁失败", Toast.LENGTH_SHORT).show()
                }
            } catch (_: Exception) {}
        }
    }
}
