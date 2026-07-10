package com.taskmod.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

class BootReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "BootReceiver"
    }

    override fun onReceive(context: Context, intent: Intent) {
        if (intent.action == Intent.ACTION_BOOT_COMPLETED ||
            intent.action == "android.intent.action.QUICKBOOT_POWERON") {

            Log.i(TAG, "设备启动完成，检查是否需要自动启动服务")

            val config = ConfigManager.load()
            if (config.autoStart) {
                Log.i(TAG, "自动启动 TaskMod 服务")
                TaskModService.start(context)
            }
        }
    }
}
