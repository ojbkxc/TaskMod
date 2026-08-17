package com.taskmod.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

class TtsReceiver : BroadcastReceiver() {

    companion object {
        private const val TAG = "TtsReceiver"
        const val ACTION_SPEAK = "com.taskmod.app.TTS_SPEAK"
        const val ACTION_STOP = "com.taskmod.app.TTS_STOP"
        const val ACTION_INIT = "com.taskmod.app.TTS_INIT"
    }

    override fun onReceive(context: Context, intent: Intent) {
        try {
            when (intent.action) {
                ACTION_SPEAK -> handleSpeak(intent)
                ACTION_STOP -> {
                    Log.d(TAG, "TTS_STOP received")
                    TtsManager.stop()
                }
                ACTION_INIT -> {
                    Log.d(TAG, "TTS_INIT received")
                    TtsManager.init(context)
                }
                else -> Log.w(TAG, "Unknown action: ${intent.action}")
            }
        } catch (t: Throwable) {
            Log.e(TAG, "onReceive error: ${t.message}", t)
        }
    }

    private fun handleSpeak(intent: Intent) {
        val text = intent.getStringExtra("text")
        if (text.isNullOrBlank()) {
            Log.w(TAG, "TTS_SPEAK: text is null or blank, ignoring")
            return
        }
        val engine = intent.getStringExtra("engine")
        val language = intent.getStringExtra("language")
        val rate = intent.getFloatExtra("rate", 1.0f)
        val pitch = intent.getFloatExtra("pitch", 1.0f)
        Log.d(TAG, "TTS_SPEAK: textLen=${text.length} engine=$engine language=$language rate=$rate pitch=$pitch")
        if (!engine.isNullOrEmpty()) {
            TtsManager.setEngineAndSpeak(text, engine, language, rate, pitch)
        } else {
            val lang = if (language.isNullOrBlank()) "system" else language
            TtsManager.speak(text, lang, rate, pitch)
        }
    }
}