package com.taskmod.app.tiles

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.widget.Toast
import com.taskmod.app.RootHelper

class ScreenshotTile : TileService() {

    override fun onStartListening() {
        super.onStartListening()
        qsTile?.let {
            it.state = Tile.STATE_ACTIVE
            it.updateTile()
        }
    }

    override fun onClick() {
        super.onClick()
        Thread {
            val (success, _) = RootHelper.executeRoot("screencap -p /sdcard/screenshot.png")
            android.os.Handler(mainLooper).post {
                Toast.makeText(this, if (success) "截屏成功" else "截屏失败", Toast.LENGTH_SHORT).show()
            }
        }.start()
    }
}
