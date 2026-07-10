package com.taskmod.app.tiles

import android.os.Build
import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import com.taskmod.app.ServerManager
import com.taskmod.app.TaskModService

class ServerToggleTile : TileService() {

    override fun onStartListening() {
        super.onStartListening()
        val manager = ServerManager(this)
        val running = manager.isRunning()
        qsTile?.let {
            it.state = if (running) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
            it.subtitle = if (running) "运行中" else "已停止"
            it.updateTile()
        }
    }

    override fun onClick() {
        super.onClick()
        val manager = ServerManager(this)
        if (manager.isRunning()) {
            TaskModService.stop(this)
            qsTile?.let {
                it.state = Tile.STATE_INACTIVE
                it.subtitle = "已停止"
                it.updateTile()
            }
        } else {
            TaskModService.start(this)
            qsTile?.let {
                it.state = Tile.STATE_ACTIVE
                it.subtitle = "启动中…"
                it.updateTile()
            }
        }
    }
}
