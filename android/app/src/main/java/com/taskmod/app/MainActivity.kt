package com.taskmod.app

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.*
import android.widget.*
import androidx.appcompat.app.AlertDialog
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.appbar.MaterialToolbar
import com.google.android.material.button.MaterialButton
import com.google.android.material.card.MaterialCardView
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import java.io.File

class MainActivity : AppCompatActivity() {

    private lateinit var serverManager: ServerManager
    private val handler = Handler(Looper.getMainLooper())

    // Views
    private lateinit var tvStatus: TextView
    private lateinit var tvStatusText: TextView
    private lateinit var statusDot: View
    private lateinit var tvAddress: TextView
    private lateinit var btnToggle: MaterialButton
    private lateinit var btnOpenPanel: MaterialButton
    private lateinit var btnScreenshot: MaterialButton
    private lateinit var btnUnlock: MaterialButton
    private lateinit var btnReboot: MaterialButton
    private lateinit var btnShare: MaterialButton
    private lateinit var tvRootStatus: TextView
    private lateinit var btnMagiskGuide: MaterialButton
    private lateinit var tvVersion: TextView
    private lateinit var toolbar: MaterialToolbar

    private var webView: WebView? = null
    private var statusCheckRunnable: Runnable? = null

    private val statusReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            updateUI()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        serverManager = ServerManager(this)
        initViews()
        setupListeners()
        checkRootStatus()
        updateUI()
        startStatusCheck()

        // 检查是否需要自动启动
        if (ConfigManager.load().autoStart) {
            TaskModService.start(this)
        }

        // 检查更新
        UpdateChecker(this).checkForUpdates()
    }

    private fun initViews() {
        toolbar = findViewById(R.id.toolbar)
        tvStatus = findViewById(R.id.tv_status)
        tvStatusText = findViewById(R.id.tv_status_text)
        statusDot = findViewById(R.id.status_dot)
        tvAddress = findViewById(R.id.tv_address)
        btnToggle = findViewById(R.id.btn_toggle)
        btnOpenPanel = findViewById(R.id.btn_open_panel)
        btnScreenshot = findViewById(R.id.btn_screenshot)
        btnUnlock = findViewById(R.id.btn_unlock)
        btnReboot = findViewById(R.id.btn_reboot)
        btnShare = findViewById(R.id.btn_share)
        tvRootStatus = findViewById(R.id.tv_root_status)
        btnMagiskGuide = findViewById(R.id.btn_magisk_guide)
        tvVersion = findViewById(R.id.tv_version)

        // 设置版本号
        try {
            val pInfo = packageManager.getPackageInfo(packageName, 0)
            tvVersion.text = "v${pInfo.versionName}"
        } catch (e: Exception) {
            tvVersion.text = "v1.0.0"
        }

        // 菜单
        toolbar.setOnMenuItemClickListener { item ->
            when (item.itemId) {
                R.id.action_settings -> {
                    startActivity(Intent(this, SettingsActivity::class.java))
                    true
                }
                R.id.action_check_update -> {
                    UpdateChecker(this).checkForUpdates(force = true)
                    true
                }
                R.id.action_about -> {
                    showAboutDialog()
                    true
                }
                else -> false
            }
        }
    }

    private fun setupListeners() {
        btnToggle.setOnClickListener {
            if (serverManager.isRunning()) {
                TaskModService.stop(this)
            } else {
                TaskModService.start(this)
            }
        }

        btnOpenPanel.setOnClickListener {
            openWebView()
        }

        btnScreenshot.setOnClickListener {
            if (!serverManager.isRunning()) {
                Toast.makeText(this, "服务未运行", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }
            Thread {
                val (success, _) = serverManager.executeCommand("screencap -p /sdcard/screenshot.png")
                handler.post {
                    Toast.makeText(this, if (success) "截屏成功" else "截屏失败", Toast.LENGTH_SHORT).show()
                }
            }.start()
        }

        btnUnlock.setOnClickListener {
            Thread {
                serverManager.executeCommand("input keyevent KEYCODE_WAKEUP")
                Thread.sleep(300)
                val (success, _) = serverManager.executeRoot("input swipe 540 1800 540 600 300")
                handler.post {
                    Toast.makeText(this, if (success) "上滑解锁已执行" else "解锁失败", Toast.LENGTH_SHORT).show()
                }
            }.start()
        }

        btnReboot.setOnClickListener {
            MaterialAlertDialogBuilder(this)
                .setTitle("重启设备")
                .setMessage("确定要重启设备吗？")
                .setPositiveButton("重启") { _, _ ->
                    Thread { serverManager.executeRoot("reboot") }.start()
                }
                .setNegativeButton("取消", null)
                .show()
        }

        btnShare.setOnClickListener {
            val urls = serverManager.getAllAccessUrls()
            val text = "TaskMod 管理面板:\n${urls.joinToString("\n")}"
            val intent = Intent(Intent.ACTION_SEND).apply {
                type = "text/plain"
                putExtra(Intent.EXTRA_TEXT, text)
            }
            startActivity(Intent.createChooser(intent, "分享面板地址"))
        }

        btnMagiskGuide.setOnClickListener {
            showMagiskGuide()
        }
    }

    private fun openWebView() {
        if (webView != null) return

        val intent = Intent(this, WebViewActivity::class.java)
        intent.putExtra("url", serverManager.getLocalUrl())
        startActivity(intent)
    }

    private fun checkRootStatus() {
        Thread {
            val result = RootHelper.checkRoot()
            val moduleInstalled = RootHelper.isMagiskModuleInstalled()
            handler.post {
                when {
                    result.hasRoot && result.method == "magisk" -> {
                        if (moduleInstalled) {
                            tvRootStatus.text = "已获取 Root (Magisk) + 模块已安装"
                            tvRootStatus.setTextColor(getColor(R.color.success))
                            btnMagiskGuide.visibility = View.GONE
                        } else {
                            tvRootStatus.text = "已获取 Root (Magisk) - 模块未安装"
                            tvRootStatus.setTextColor(getColor(R.color.warning))
                            btnMagiskGuide.text = "安装模块（推荐）"
                            btnMagiskGuide.visibility = View.VISIBLE
                            // 首次启动自动弹出引导
                            val prefs = getSharedPreferences("taskmod", MODE_PRIVATE)
                            if (prefs.getBoolean("show_guide_first", true)) {
                                prefs.edit().putBoolean("show_guide_first", false).apply()
                                startActivity(Intent(this, MagiskGuideActivity::class.java))
                            }
                        }
                    }
                    result.hasRoot -> {
                        tvRootStatus.text = "已获取 Root (su) - 建议使用 Magisk"
                        tvRootStatus.setTextColor(getColor(R.color.success))
                        btnMagiskGuide.visibility = View.VISIBLE
                    }
                    else -> {
                        tvRootStatus.text = "未获取 Root - 设备控制功能不可用"
                        tvRootStatus.setTextColor(getColor(R.color.error))
                        btnMagiskGuide.visibility = View.VISIBLE
                    }
                }
            }
        }.start()
    }

    private fun updateUI() {
        val running = serverManager.isRunning()
        tvStatusText.text = if (running) "运行中" else "已停止"
        statusDot.setBackgroundResource(
            if (running) R.drawable.status_dot_running else R.drawable.status_dot_stopped
        )
        if (running) {
            // 显示所有可用地址
            val urls = serverManager.getAllAccessUrls()
            tvAddress.text = urls.joinToString("\n")
        } else {
            tvAddress.text = "http://--:${serverManager.port}"
        }
        btnToggle.text = if (running) "停止服务" else "启动服务"
        btnToggle.setBackgroundColor(getColor(if (running) R.color.error else R.color.primary))
    }

    private fun startStatusCheck() {
        statusCheckRunnable = object : Runnable {
            override fun run() {
                updateUI()
                handler.postDelayed(this, 5000)
            }
        }
        handler.post(statusCheckRunnable!!)

        // 注册广播接收器
        val filter = IntentFilter("com.taskmod.STATUS_CHANGED")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(statusReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(statusReceiver, filter)
        }
    }

    private fun showMagiskGuide() {
        startActivity(Intent(this, MagiskGuideActivity::class.java))
    }

    private fun showAboutDialog() {
        MaterialAlertDialogBuilder(this)
            .setTitle("关于 TaskMod")
            .setMessage("""
                TaskMod - Android 设备 AI 自动化引擎
                
                版本: ${tvVersion.text}
                项目: https://github.com/${TaskModApp.GITHUB_REPO}
                
                功能：实时投屏、AI对话控制、脚本调度、设备监控
            """.trimIndent())
            .setPositiveButton("确定", null)
            .show()
    }

    override fun onResume() {
        super.onResume()
        updateUI()
    }

    override fun onDestroy() {
        statusCheckRunnable?.let { handler.removeCallbacks(it) }
        try { unregisterReceiver(statusReceiver) } catch (e: Exception) {}
        super.onDestroy()
    }
}
