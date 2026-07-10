package com.taskmod.app

import android.Manifest
import android.annotation.SuppressLint
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageManager
import android.content.res.ColorStateList
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.View
import android.webkit.*
import android.widget.*
import androidx.activity.result.contract.ActivityResultContracts
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.ContextCompat
import androidx.lifecycle.lifecycleScope
import com.google.android.material.button.MaterialButton
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class MainActivity : AppCompatActivity() {

    private lateinit var serverManager: ServerManager
    private val handler = Handler(Looper.getMainLooper())

    // Views
    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar
    private lateinit var placeholder: LinearLayout
    private lateinit var tvStatusText: TextView
    private lateinit var statusDot: View
    private lateinit var tvAddress: TextView
    private lateinit var btnToggle: MaterialButton
    private lateinit var btnMenu: ImageButton

    private var statusCheckRunnable: Runnable? = null
    private var isPageLoaded = false

    private val notificationPermissionLauncher =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { _ -> }

    private val statusReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            updateUI()
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        serverManager = ServerManager.getInstance(this)
        initViews()
        setupWebView()
        setupListeners()
        checkRootStatus()
        updateUI()
        startStatusCheck()

        if (ConfigManager.load().autoStart) {
            TaskModService.start(this)
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED
            ) {
                notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }

        UpdateChecker(this).checkForUpdates()
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun setupWebView() {
        webView.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            allowFileAccess = true
            allowContentAccess = true
            mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
            useWideViewPort = true
            loadWithOverviewMode = true
            setSupportZoom(true)
            builtInZoomControls = true
            displayZoomControls = false
            cacheMode = WebSettings.LOAD_NO_CACHE
            mediaPlaybackRequiresUserGesture = false
            userAgentString = "TaskMod-Android/${getVersionName()}"
        }

        webView.webChromeClient = object : WebChromeClient() {
            override fun onProgressChanged(view: WebView?, newProgress: Int) {
                progressBar.progress = newProgress
                progressBar.visibility = if (newProgress < 100) View.VISIBLE else View.GONE
            }
        }

        webView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(view: WebView?, request: WebResourceRequest?): Boolean {
                val requestUrl = request?.url.toString()
                if (requestUrl.contains("localhost") || requestUrl.contains("127.0.0.1")) {
                    return false
                }
                startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(requestUrl)))
                return true
            }

            override fun onPageFinished(view: WebView?, url: String?) {
                super.onPageFinished(view, url)
                isPageLoaded = true
            }
        }
    }

    private fun initViews() {
        webView = findViewById(R.id.webview)
        progressBar = findViewById(R.id.progress_bar)
        placeholder = findViewById(R.id.placeholder)
        tvStatusText = findViewById(R.id.tv_status_text)
        statusDot = findViewById(R.id.status_dot)
        tvAddress = findViewById(R.id.tv_address)
        btnToggle = findViewById(R.id.btn_toggle)
        btnMenu = findViewById(R.id.btn_menu)
    }

    private fun setupListeners() {
        btnToggle.setOnClickListener {
            if (serverManager.isRunning()) {
                TaskModService.stop(this)
                isPageLoaded = false
            } else {
                TaskModService.start(this)
                // 等服务启动后加载页面
                handler.postDelayed({ loadWebView() }, 1500)
            }
        }

        btnMenu.setOnClickListener { showPopupMenu(it) }
    }

    private fun showPopupMenu(anchor: View) {
        val popup = android.widget.PopupMenu(this, anchor)
        popup.menu.add(0, 1, 0, "截屏")
        popup.menu.add(0, 2, 0, "上滑解锁")
        popup.menu.add(0, 3, 0, "重启设备")
        popup.menu.add(0, 4, 0, "分享面板地址")
        popup.menu.add(0, 5, 0, "Magisk 模块引导")
        popup.menu.add(0, 6, 0, "设置")
        popup.menu.add(0, 7, 0, "检查更新")
        popup.menu.add(0, 8, 0, "关于")
        popup.setOnMenuItemClickListener { item ->
            when (item.itemId) {
                1 -> doScreenshot()
                2 -> doUnlock()
                3 -> doReboot()
                4 -> doShare()
                5 -> startActivity(Intent(this, MagiskGuideActivity::class.java))
                6 -> startActivity(Intent(this, SettingsActivity::class.java))
                7 -> UpdateChecker(this).checkForUpdates(force = true)
                8 -> showAboutDialog()
            }
            true
        }
        popup.show()
    }

    private fun doScreenshot() {
        if (!serverManager.isRunning()) {
            Toast.makeText(this, "服务未运行", Toast.LENGTH_SHORT).show()
            return
        }
        lifecycleScope.launch(Dispatchers.IO) {
            val (success, _) = serverManager.executeCommand("screencap -p /sdcard/screenshot.png")
            withContext(Dispatchers.Main) {
                Toast.makeText(this@MainActivity, if (success) "截屏成功" else "截屏失败", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun doUnlock() {
        lifecycleScope.launch(Dispatchers.IO) {
            serverManager.executeCommand("input keyevent KEYCODE_WAKEUP")
            delay(300)
            val (success, _) = serverManager.executeRoot("input swipe 540 1800 540 600 300")
            withContext(Dispatchers.Main) {
                Toast.makeText(this@MainActivity, if (success) "上滑解锁已执行" else "解锁失败", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun doReboot() {
        android.app.AlertDialog.Builder(this)
            .setTitle("重启设备")
            .setMessage("确定要重启设备吗？")
            .setPositiveButton("重启") { _, _ ->
                lifecycleScope.launch(Dispatchers.IO) { serverManager.executeRoot("reboot") }
            }
            .setNegativeButton("取消", null)
            .show()
    }

    private fun doShare() {
        val urls = serverManager.getAllAccessUrls()
        val text = "TaskMod 管理面板:\n${urls.joinToString("\n")}"
        val intent = Intent(Intent.ACTION_SEND).apply {
            type = "text/plain"
            putExtra(Intent.EXTRA_TEXT, text)
        }
        startActivity(Intent.createChooser(intent, "分享面板地址"))
    }

    private fun loadWebView() {
        if (serverManager.isRunning()) {
            val url = serverManager.getLocalUrl()
            webView.loadUrl(url)
            webView.visibility = View.VISIBLE
            placeholder.visibility = View.GONE
        } else {
            webView.visibility = View.GONE
            placeholder.visibility = View.VISIBLE
        }
    }

    private fun checkRootStatus() {
        lifecycleScope.launch(Dispatchers.IO) {
            val result = RootHelper.checkRoot()
            val moduleInstalled = RootHelper.isMagiskModuleInstalled()
            withContext(Dispatchers.Main) {
                if (!result.hasRoot) {
                    // 无Root时在地址栏提示
                }
                val prefs = getSharedPreferences("taskmod", MODE_PRIVATE)
                if (result.hasRoot && result.method == "magisk" && !moduleInstalled
                    && prefs.getBoolean("show_guide_first", true)) {
                    prefs.edit().putBoolean("show_guide_first", false).apply()
                    startActivity(Intent(this@MainActivity, MagiskGuideActivity::class.java))
                }
            }
        }
    }

    private fun updateUI() {
        val running = serverManager.isRunning()
        tvStatusText.text = if (running) "运行中" else "已停止"
        tvStatusText.setTextColor(getColor(if (running) R.color.success else R.color.error))
        statusDot.setBackgroundResource(
            if (running) R.drawable.status_dot_running else R.drawable.status_dot_stopped
        )
        if (running) {
            tvAddress.text = serverManager.getLocalUrl()
            btnToggle.text = "停止"
            btnToggle.backgroundTintList = ColorStateList.valueOf(getColor(R.color.error))
            // 自动加载 WebView
            if (!isPageLoaded) loadWebView()
        } else {
            tvAddress.text = "--"
            btnToggle.text = "启动"
            btnToggle.backgroundTintList = ColorStateList.valueOf(getColor(R.color.primary))
            webView.visibility = View.GONE
            placeholder.visibility = View.VISIBLE
        }
    }

    private fun startStatusCheck() {
        statusCheckRunnable = object : Runnable {
            override fun run() {
                updateUI()
                handler.postDelayed(this, 5000)
            }
        }
        handler.post(statusCheckRunnable!!)

        val filter = IntentFilter("com.taskmod.STATUS_CHANGED")
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(statusReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(statusReceiver, filter)
        }
    }

    private fun showAboutDialog() {
        android.app.AlertDialog.Builder(this)
            .setTitle("关于 TaskMod")
            .setMessage(
                """
                TaskMod - Android 设备 AI 自动化引擎
                
                版本: v${getVersionName()}
                项目: https://github.com/${TaskModApp.GITHUB_REPO}
                
                功能：实时投屏、AI对话控制、脚本调度、设备监控
                """.trimIndent()
            )
            .setPositiveButton("确定", null)
            .show()
    }

    private fun getVersionName(): String {
        return try {
            packageManager.getPackageInfo(packageName, 0).versionName ?: "1.0.0"
        } catch (e: Exception) {
            "1.0.0"
        }
    }

    override fun onResume() {
        super.onResume()
        updateUI()
    }

    override fun onBackPressed() {
        if (webView.canGoBack()) {
            webView.goBack()
        } else {
            super.onBackPressed()
        }
    }

    override fun onDestroy() {
        statusCheckRunnable?.let { handler.removeCallbacks(it) }
        try { unregisterReceiver(statusReceiver) } catch (e: Exception) {}
        super.onDestroy()
    }
}
