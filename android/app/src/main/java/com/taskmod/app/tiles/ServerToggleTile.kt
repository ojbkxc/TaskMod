package com.taskmod.app.tiles

import android.service.quicksettings.Tile
import android.service.quicksettings.TileService
import android.widget.Toast
import com.taskmod.app.ServerManager
import com.taskmod.app.TaskModService
import kotlinx.coroutines.*

class ServerToggleTile : TileService() {

    private val scope = CoroutineScope(SupervisorJob() + Dispatchers.IO)

    override fun onStartListening() {
        super.onStartListening()
        scope.launch {
            val manager = ServerManager.getInstance(this@ServerToggleTile)
            val running = manager.isRunning()
            qsTile?.let {
                it.state = if (running) Tile.STATE_ACTIVE else Tile.STATE_INACTIVE
                it.subtitle = if (running) "运行中" else "已停止"
                it.updateTile()
            }
        }
    }

    override fun onClick() {
        super.onClick()
        scope.launch {
            val manager = ServerManager.getInstance(this@ServerToggleTile)
            val running = manager.isRunning()

            if (running) {
                // 停止服务
                withContext(Dispatchers.Main) {
                    qsTile?.let {
                        it.state = Tile.STATE_INACTIVE
                        it.subtitle = "停止中…"
                        it.updateTile()
                    }
                }
                TaskModService.stop(this@ServerToggleTile)
                withContext(Dispatchers.Main) {
                    qsTile?.let {
                        it.state = Tile.STATE_INACTIVE
                        it.subtitle = "已停止"
                        it.updateTile()
                    }
                    Toast.makeText(this@ServerToggleTile, "服务已停止", Toast.LENGTH_SHORT).show()
                }
            } else {
                // 启动服务
                withContext(Dispatchers.Main) {
                    qsTile?.let {
                        it.state = Tile.STATE_UNAVAILABLE
                        it.subtitle = "启动中…"
                        it.updateTile()
                    }
                    Toast.makeText(this@ServerToggleTile, "正在启动服务…", Toast.LENGTH_SHORT).show()
                }
                TaskModService.start(this@ServerToggleTile)

                // 轮询等待服务启动（最多 10 秒）
                var started = false
                for (i in 1..20) {
                    delay(500)
                    if (manager.isRunning()) {
                        started = true
                        break
                    }
                }
                withContext(Dispatchers.Main) {
                    if (started) {
                        qsTile?.let {
                            it.state = Tile.STATE_ACTIVE
                            it.subtitle = "运行中"
                            it.updateTile()
                        }
                        Toast.makeText(this@ServerToggleTile, "服务已启动", Toast.LENGTH_SHORT).show()
                    } else {
                        qsTile?.let {
                            it.state = Tile.STATE_INACTIVE
                            it.subtitle = "启动失败"
                            it.updateTile()
                        }
                        Toast.makeText(this@ServerToggleTile, "服务启动超时", Toast.LENGTH_LONG).show()
                    }
                }
            }
        }
    }

    override fun onDestroy() {
        scope.cancel()
        super.onDestroy()
    }
}