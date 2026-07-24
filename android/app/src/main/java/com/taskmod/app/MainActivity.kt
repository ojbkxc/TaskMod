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
import android.util.Log
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

    companion object {
        private const val TAG = "MainActivity"
        private const val STATUS_CHECK_INTERVAL = 3000L
    }

    private lateinit var serverManager: ServerManager
    private val handler = Handler(Looper.getMainLooper())

    // Views
    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar
    private lateinit var placeholder: LinearLayout
    private lateinit var tvPlaceholderHint: TextView
    private lateinit var tvStatusText: TextView
    private lateinit var statusDot: View
    private lateinit var tvAddress: TextView
    private lateinit var btnToggle: MaterialButton
    private lateinit var btnMenu: ImageButton

    private var statusCheckRunnable: Runnable? = null
    private var isPageLoaded = false
    private var isToggling = false
    private var webViewRetryCount = 0
    private val maxRetryCount = 3

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
        updateUIAsync()
        startStatusCheck()

        // 先做一次状态检查，再根据状态决定是否自动启动
        updateUI()

        if (ConfigManager.load().autoStart && !serverManager.isRunning()) {
            TaskModService.start(this)
        }

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            if (ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS)
                != PackageManager.PERMISSION_GRANTED
            ) {
                notificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
            }
        }

        UpdateChecker.checkForUpdates(this)
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun setupWebView() {
        webView.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            allowFileAccess = false
            allowContentAccess = false
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
                val requestUrl = request?.url
                if (requestUrl != null) {
                    val host = requestUrl.host ?: ""
                    if (host in setOf("localhost", "127.0.0.1", "10.0.2.2") ||
                    host.startsWith("10.") || host.startsWith("192.168.") ||
                    host.startsWith("172.16.") || host.startsWith("172.17.") ||
                    host.startsWith("172.18.") || host.startsWith("172.19.") ||
                    host.startsWith("172.20.") || host.startsWith("172.21.") ||
                    host.startsWith("172.22.") || host.startsWith("172.23.") ||
                    host.startsWith("172.24.") || host.startsWith("172.25.") ||
                    host.startsWith("172.26.") || host.startsWith("172.27.") ||
                    host.startsWith("172.28.") || host.startsWith("172.29.") ||
                    host.startsWith("172.30.") || host.startsWith("172.31.")) {
                        return false
                    }
                }
                val uri = request?.url ?: return true
                startActivity(Intent(Intent.ACTION_VIEW, uri))
                return true
            }

            override fun onPageFinished(view: WebView?, url: String?) {
                super.onPageFinished(view, url)
                isPageLoaded = true
                webViewRetryCount = 0
            }

            override fun onReceivedError(view: WebView?, request: WebResourceRequest?, error: WebResourceError?) {
                super.onReceivedError(view, request, error)
                // 只在主资源加载失败时处理（忽略子资源错误）
                if (request?.isForMainFrame == true) {
                    isPageLoaded = false
                    Log.w("MainActivity", "WebView 加载失败: ${error?.description} (code=${error?.errorCode})")
                    showWebViewError()
                }
            }
        }
    }

    /**
     * WebView 加载失败时显示错误提示，点击可重试
     */
    private fun showWebViewError() {
        webView.visibility = View.GONE
        placeholder.visibility = View.VISIBLE
        tvPlaceholderHint.text = if (webViewRetryCount < maxRetryCount) {
            "连接失败，点击重试 ($webViewRetryCount/$maxRetryCount)"
        } else {
            "连接失败，请检查服务是否正常运行\n点击重试"
        }
    }

    private fun initViews() {
        webView = findViewById(R.id.webview)
        progressBar = findViewById(R.id.progress_bar)
        placeholder = findViewById(R.id.placeholder)
        tvPlaceholderHint = findViewById(R.id.tv_placeholder_hint)
        tvStatusText = findViewById(R.id.tv_status_text)
        statusDot = findViewById(R.id.status_dot)
        tvAddress = findViewById(R.id.tv_address)
        btnToggle = findViewById(R.id.btn_toggle)
        btnMenu = findViewById(R.id.btn_menu)
    }

    private fun setupListeners() {
        btnToggle.setOnClickListener {
            if (isToggling) {
                Toast.makeText(this, "操作进行中，请稍候…", Toast.LENGTH_SHORT).show()
                return@setOnClickListener
            }
            isToggling = true
            btnToggle.isEnabled = false

            lifecycleScope.launch(Dispatchers.IO) {
                val running = serverManager.isRunning()
                withContext(Dispatchers.Main) {
                    if (running) {
                        btnToggle.text = "停止中…"
                        Toast.makeText(this@MainActivity, "正在停止服务…", Toast.LENGTH_SHORT).show()
                    } else {
                        btnToggle.text = "启动中…"
                        Toast.makeText(this@MainActivity, "正在启动服务…", Toast.LENGTH_SHORT).show()
                    }
                }

                if (running) {
                    TaskModService.stop(this@MainActivity)
                    withContext(Dispatchers.Main) {
                        isPageLoaded = false
                        webViewRetryCount = 0
                        Toast.makeText(this@MainActivity, "服务已停止", Toast.LENGTH_SHORT).show()
                    }
                } else {
                    TaskModService.start(this@MainActivity)
                    // 等待服务启动，最多轮询 10 秒
                    var started = false
                    for (i in 1..20) {
                        delay(500)
                        if (serverManager.isRunning()) {
                            started = true
                            break
                        }
                    }
                    withContext(Dispatchers.Main) {
                        if (started) {
                            Toast.makeText(this@MainActivity, "服务已启动", Toast.LENGTH_SHORT).show()
                            loadWebView()
                        } else {
                            Toast.makeText(this@MainActivity, "服务启动超时，请检查日志", Toast.LENGTH_LONG).show()
                            showWebViewError()
                        }
                    }
                }

                withContext(Dispatchers.Main) {
                    isToggling = false
                    btnToggle.isEnabled = true
                    updateUIAsync()
                }
            }
        }

        btnMenu.setOnClickListener { showPopupMenu(it) }

        placeholder.setOnClickListener {
            retryLoadWebView()
        }
    }

    /**
     * 重试加载 WebView
     */
    private fun retryLoadWebView() {
        if (webViewRetryCount >= maxRetryCount) {
            webViewRetryCount = 0 // 重置计数器允许继续重试
        }
        webViewRetryCount++
        tvPlaceholderHint.text = "正在连接…"
        lifecycleScope.launch(Dispatchers.IO) {
            // 先尝试发现局域网服务
            val serverUrl = serverManager.findAvailableServer()
            withContext(Dispatchers.Main) {
                if (serverUrl != null) {
                    Log.i("MainActivity", "发现可用服务: $serverUrl")
                    webView.loadUrl(serverUrl)
                    webView.visibility = View.VISIBLE
                    placeholder.visibility = View.GONE
                    tvAddress.text = serverUrl
                } else if (serverManager.state == ServerManager.ServerState.RUNNING) {
                    // 本地服务正在运行，直接加载
                    loadWebViewInternal()
                } else {
                    tvPlaceholderHint.text = "未发现可用服务\n点击重试或启动服务"
                }
            }
        }
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

        // 服务未运行时禁用需要 Root 的操作
        lifecycleScope.launch(Dispatchers.IO) {
            val running = serverManager.isRunning()
            withContext(Dispatchers.Main) {
                popup.menu.findItem(1)?.isEnabled = running
                popup.menu.findItem(2)?.isEnabled = running
                popup.menu.findItem(3)?.isEnabled = running
            }
        }

        popup.setOnMenuItemClickListener { item ->
            when (item.itemId) {
                1 -> doScreenshot()
                2 -> doUnlock()
                3 -> doReboot()
                4 -> doShare()
                5 -> startActivity(Intent(this, MagiskGuideActivity::class.java))
                6 -> startActivity(Intent(this, SettingsActivity::class.java))
                7 -> UpdateChecker.checkForUpdates(this, force = true)
                8 -> showAboutDialog()
            }
            true
        }
        popup.show()
    }

    private fun doScreenshot() {
        lifecycleScope.launch(Dispatchers.IO) {
            if (!serverManager.isRunning()) {
                withContext(Dispatchers.Main) {
                    Toast.makeText(this@MainActivity, "服务未运行", Toast.LENGTH_SHORT).show()
                }
                return@launch
            }
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
        lifecycleScope.launch(Dispatchers.IO) {
            val urls = serverManager.getAllAccessUrls()
            withContext(Dispatchers.Main) {
                val text = "TaskMod 管理面板:\n${urls.joinToString("\n")}"
                val intent = Intent(Intent.ACTION_SEND).apply {
                    type = "text/plain"
                    putExtra(Intent.EXTRA_TEXT, text)
                }
                startActivity(Intent.createChooser(intent, "分享面板地址"))
            }
        }
    }

    private fun loadWebView() {
        lifecycleScope.launch(Dispatchers.IO) {
            val running = serverManager.isRunning()
            withContext(Dispatchers.Main) {
                if (running) {
                    loadWebViewInternal()
                } else {
                    webView.visibility = View.GONE
                    placeholder.visibility = View.VISIBLE
                    tvPlaceholderHint.text = "服务未运行\n点击启动服务"
                }
            }
        }
    }

    private fun loadWebViewInternal() {
        val url = serverManager.getLocalUrl()
        webView.loadUrl(url)
        webView.visibility = View.VISIBLE
        placeholder.visibility = View.GONE
        tvPlaceholderHint.text = "正在加载…"
    }

    private fun checkRootStatus() {
        lifecycleScope.launch(Dispatchers.IO) {
            val result = RootHelper.checkRoot()
            val moduleInstalled = RootHelper.isMagiskModuleInstalled()
            withContext(Dispatchers.Main) {
                val prefs = getSharedPreferences("taskmod", MODE_PRIVATE)
                if (result.hasRoot && result.method == "magisk" && !moduleInstalled
                    && prefs.getBoolean("show_guide_first", true)
                ) {
                    prefs.edit().putBoolean("show_guide_first", false).apply()
                    startActivity(Intent(this@MainActivity, MagiskGuideActivity::class.java))
                }
            }
        }
    }

    private fun updateUIAsync() {
        lifecycleScope.launch(Dispatchers.IO) {
            val running = serverManager.isRunning()
            withContext(Dispatchers.Main) {
                updateUIInternal(running)
            }
        }
    }

    private fun updateUI() {
        updateUIAsync()
    }

    private fun updateUIInternal(running: Boolean) {
        if (isToggling) return // 操作进行中，不更新 UI

        tvStatusText.text = if (running) "运行中" else "已停止"
        tvStatusText.setTextColor(
            ContextCompat.getColor(this, if (running) R.color.success else R.color.error)
        )
        statusDot.setBackgroundResource(
            if (running) R.drawable.status_dot_running
            else R.drawable.status_dot_stopped
        )
        if (running) {
            tvAddress.text = serverManager.getLocalUrl()
            btnToggle.text = "停止"
            btnToggle.backgroundTintList = ColorStateList.valueOf(
                ContextCompat.getColor(this, R.color.error)
            )
            if (!isPageLoaded) loadWebView()
        } else {
            tvAddress.text = "--"
            btnToggle.text = "启动"
            btnToggle.backgroundTintList = ColorStateList.valueOf(
                ContextCompat.getColor(this, R.color.primary)
            )
            webView.visibility = View.GONE
            placeholder.visibility = View.VISIBLE
            tvPlaceholderHint.text = "服务未运行\n点击启动服务或下拉刷新"
        }
    }

    private fun tryDiscoverServer() {
        tvPlaceholderHint.text = "正在搜索局域网服务…"
        lifecycleScope.launch(Dispatchers.IO) {
            val serverUrl = serverManager.findAvailableServer()
            withContext(Dispatchers.Main) {
                if (serverUrl != null) {
                    Log.i(TAG, "发现可用服务: $serverUrl")
                    webView.loadUrl(serverUrl)
                    webView.visibility = View.VISIBLE
                    placeholder.visibility = View.GONE
                    tvAddress.text = serverUrl
                    tvStatusText.text = "已连接"
                    tvStatusText.setTextColor(
                        ContextCompat.getColor(this@MainActivity, R.color.success)
                    )
                    statusDot.setBackgroundResource(R.drawable.status_dot_running)
                } else {
                    tvPlaceholderHint.text = "未发现可用服务\n点击重试或启动本机服务"
                }
            }
        }
    }

    private fun startStatusCheck() {
        statusCheckRunnable = object : Runnable {
            override fun run() {
                updateUI()
                handler.postDelayed(this, STATUS_CHECK_INTERVAL)
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