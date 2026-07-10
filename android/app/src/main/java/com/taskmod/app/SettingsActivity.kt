package com.taskmod.app

import android.os.Bundle
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.switchmaterial.SwitchMaterial

class SettingsActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // 简单设置页面，使用代码创建布局
        val scrollView = android.widget.ScrollView(this)
        val layout = android.widget.LinearLayout(this).apply {
            orientation = android.widget.LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
        }

        // 自动启动开关
        val prefs = getSharedPreferences("taskmod", MODE_PRIVATE)
        val autoStartSwitch = SwitchMaterial(this).apply {
            text = "开机自动启动服务"
            isChecked = prefs.getBoolean("auto_start", true)
            setOnCheckedChangeListener { _, isChecked ->
                prefs.edit().putBoolean("auto_start", isChecked).apply()
                Toast.makeText(this@SettingsActivity, if (isChecked) "已开启自动启动" else "已关闭自动启动", Toast.LENGTH_SHORT).show()
            }
        }
        layout.addView(autoStartSwitch)

        // 服务端口显示
        val portText = android.widget.TextView(this).apply {
            text = "服务端口: ${TaskModApp.PORT}"
            textSize = 16f
            setPadding(0, 32, 0, 0)
        }
        layout.addView(portText)

        scrollView.addView(layout)
        setContentView(scrollView)

        // 简单标题栏
        supportActionBar?.title = "设置"
        supportActionBar?.setDisplayHomeAsUpEnabled(true)
    }

    override fun onSupportNavigateUp(): Boolean {
        onBackPressed()
        return true
    }
}
