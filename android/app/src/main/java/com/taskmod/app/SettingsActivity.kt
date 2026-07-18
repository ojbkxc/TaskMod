package com.taskmod.app

import android.os.Bundle
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.button.MaterialButton
import com.google.android.material.switchmaterial.SwitchMaterial
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class SettingsActivity : AppCompatActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        val rootLayout = android.widget.LinearLayout(this).apply {
            orientation = android.widget.LinearLayout.VERTICAL
        }

        val toolbar = MaterialToolbar(this).apply {
            title = "设置"
            setTitleTextColor(ContextCompat.getColor(this@SettingsActivity, R.color.on_surface))
            setBackgroundColor(ContextCompat.getColor(this@SettingsActivity, R.color.surface))
            setNavigationIcon(android.R.drawable.ic_menu_revert)
            setNavigationOnClickListener { finish() }
        }
        rootLayout.addView(toolbar)

        val scrollView = android.widget.ScrollView(this)
        val layout = android.widget.LinearLayout(this).apply {
            orientation = android.widget.LinearLayout.VERTICAL
            setPadding(32, 32, 32, 32)
        }

        val config = ConfigManager.load()

        // === 自动启动 ===
        layout.addView(SwitchMaterial(this).apply {
            text = "开机自动启动服务"
            isChecked = config.autoStart
            setOnCheckedChangeListener { _, checked ->
                ConfigManager.update { copy(autoStart = checked) }
                Toast.makeText(this@SettingsActivity, if (checked) "已开启" else "已关闭", Toast.LENGTH_SHORT).show()
            }
        })

        layout.addView(makeDivider())

        // === 服务端口 ===
        layout.addView(makeLabel("服务端口"))
        layout.addView(makeHint("默认 9527，修改后需重启服务生效"))
        val portEdit = android.widget.EditText(this).apply {
            setText(config.port.toString())
            hint = "9527"
            inputType = android.text.InputType.TYPE_CLASS_NUMBER
            textSize = 14f
            setPadding(16, 12, 16, 12)
        }
        layout.addView(portEdit)

        // === 自定义 IP ===
        layout.addView(makeLabel("自定义 IP"))
        layout.addView(makeHint("留空则自动检测，如 192.168.1.100"))
        val ipEdit = android.widget.EditText(this).apply {
            setText(config.customIp)
            hint = "192.168.1.100"
            inputType = android.text.InputType.TYPE_CLASS_TEXT
            textSize = 14f
            setPadding(16, 12, 16, 12)
        }
        layout.addView(ipEdit)

        // === 自定义域名/完整 URL ===
        layout.addView(makeLabel("自定义域名 / URL"))
        layout.addView(makeHint("支持域名或完整地址，留空则使用上方 IP+端口\n如 http://myphone.ddns.net 或 http://myphone.ddns.net:8080"))
        val urlEdit = android.widget.EditText(this).apply {
            setText(config.customUrl)
            hint = "http://myphone.ddns.net"
            inputType = android.text.InputType.TYPE_CLASS_TEXT
            textSize = 14f
            setPadding(16, 12, 16, 12)
        }
        layout.addView(urlEdit)

        // === 保存按钮 + 重启按钮 ===
        val buttonRow = android.widget.LinearLayout(this).apply {
            orientation = android.widget.LinearLayout.HORIZONTAL
            layoutParams = android.widget.LinearLayout.LayoutParams(
                android.widget.LinearLayout.LayoutParams.MATCH_PARENT,
                android.widget.LinearLayout.LayoutParams.WRAP_CONTENT
            ).apply { topMargin = 24 }
        }

        buttonRow.addView(MaterialButton(this).apply {
            text = "保存设置"
            layoutParams = android.widget.LinearLayout.LayoutParams(
                0, android.widget.LinearLayout.LayoutParams.WRAP_CONTENT, 1f
            ).apply { marginEnd = 8 }
            setOnClickListener {
                val port = portEdit.text.toString().trim().toIntOrNull() ?: TaskModApp.DEFAULT_PORT
                if (port < 1024 || port > 65535) {
                    Toast.makeText(this@SettingsActivity, "端口范围: 1024-65535", Toast.LENGTH_SHORT).show()
                    return@setOnClickListener
                }
                ConfigManager.update {
                    copy(
                        port = port,
                        customIp = ipEdit.text.toString().trim(),
                        customUrl = urlEdit.text.toString().trim()
                    )
                }
                Toast.makeText(this@SettingsActivity, "已保存，重启服务生效", Toast.LENGTH_SHORT).show()
            }
        })

        buttonRow.addView(MaterialButton(this).apply {
            text = "保存并重启"
            layoutParams = android.widget.LinearLayout.LayoutParams(
                0, android.widget.LinearLayout.LayoutParams.WRAP_CONTENT, 1f
            ).apply { marginStart = 8 }
            setOnClickListener {
                val port = portEdit.text.toString().trim().toIntOrNull() ?: TaskModApp.DEFAULT_PORT
                if (port < 1024 || port > 65535) {
                    Toast.makeText(this@SettingsActivity, "端口范围: 1024-65535", Toast.LENGTH_SHORT).show()
                    return@setOnClickListener
                }
                ConfigManager.update {
                    copy(
                        port = port,
                        customIp = ipEdit.text.toString().trim(),
                        customUrl = urlEdit.text.toString().trim()
                    )
                }
                Toast.makeText(this@SettingsActivity, "正在重启服务…", Toast.LENGTH_SHORT).show()
                lifecycleScope.launch(Dispatchers.IO) {
                    TaskModService.stop(this@SettingsActivity)
                    kotlinx.coroutines.delay(1000)
                    TaskModService.start(this@SettingsActivity)
                    withContext(Dispatchers.Main) {
                        Toast.makeText(this@SettingsActivity, "服务已重启", Toast.LENGTH_SHORT).show()
                    }
                }
            }
        })
        layout.addView(buttonRow)

        layout.addView(makeDivider())

        // === 当前所有可用地址 ===
        layout.addView(makeLabel("当前可用地址"))
        val serverManager = ServerManager.getInstance(this)
        for (url in serverManager.getAllAccessUrls()) {
            layout.addView(android.widget.TextView(this).apply {
                text = url
                textSize = 13f
                setPadding(0, 4, 0, 4)
                setTextIsSelectable(true)
            })
        }

        scrollView.addView(layout)
        rootLayout.addView(scrollView)
        setContentView(rootLayout)
    }

    private fun makeLabel(text: String) = android.widget.TextView(this).apply {
        this.text = text; textSize = 15f; setPadding(0, 24, 0, 0)
        setTypeface(null, android.graphics.Typeface.BOLD)
    }

    private fun makeHint(text: String) = android.widget.TextView(this).apply {
        this.text = text; textSize = 12f; setPadding(0, 4, 0, 8)
        setTextColor(ContextCompat.getColor(this@SettingsActivity, R.color.text_secondary))
    }

    private fun makeDivider() = android.view.View(this).apply {
        layoutParams = android.widget.LinearLayout.LayoutParams(
            android.widget.LinearLayout.LayoutParams.MATCH_PARENT, 1
        ).apply { topMargin = 24; bottomMargin = 24 }
        setBackgroundColor(ContextCompat.getColor(this@SettingsActivity, R.color.surface_variant))
    }
}