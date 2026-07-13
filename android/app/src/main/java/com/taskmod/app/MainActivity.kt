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
        private const val STATUS_CHECK_INTERVAL = 5000L  // 5秒轮询一次
        private const val WEBVIEW_LOAD_DELAY = 800L       // WebView 加载延迟
    }

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
    private var lastKnownRunning = false   // 上一次已知的运行状态，用于检测状态变化
    private var webViewLoadPending = false  // 是否有待执行的 WebView 加载

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
                webViewLoadPending = false
            }

            override fun onReceivedError(
                view: WebView?,
                request: WebResourceRequest?,
                error: WebResourceError?
            ) {
                super.onReceivedError(view, request, error)
                if (request?.isForMainFrame == true) {
                    Log.w(TAG, "WebView 加载失败: ${error?.errorCode} ${error?.description}")
                    // 主页面加载失败时不标记为已加载，下次 updateUI 会重试
                    isPageLoaded = false
                    webViewLoadPending = false
                }
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
                webViewLoadPending = false
            } else {
                TaskModService.start(this)
                // 不在这里硬编码延迟，而是通过状态轮询来加载 WebView
                webViewLoadPending = true
            }
        }

        btnMenu.setOnClickListener { showPopupMenu(it) }

        placeholder.setOnClickListener {
            tryDiscoverServer()
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
            Log.i(TAG, "加载 WebView: $url")
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

        // 检测状态变化：从非运行变为运行 → 需要加载 WebView
        // 从运行变为非运行 → 重置 isPageLoaded
        if (running && !lastKnownRunning) {
            // 服务刚变为运行状态
            isPageLoaded = false
        }
        if (!running && lastKnownRunning) {
            // 服务刚停止（包括崩溃/外部停止）
            isPageLoaded = false
            webViewLoadPending = false
        }
        lastKnownRunning = running

        tvStatusText.text = if (running) "运行中" else "已停止"
        tvStatusText.setTextColor(getColor(if (running) R.color.success else R.color.error))
        statusDot.setBackgroundResource(
            if (running) R.drawable.status_dot_running else R.drawable.status_dot_stopped
        )
        if (running) {
            tvAddress.text = serverManager.getLocalUrl()
            btnToggle.text = "停止"
            btnToggle.backgroundTintList = ColorStateList.valueOf(getColor(R.color.error))
            if (!isPageLoaded) loadWebView()
        } else {
            tvAddress.text = "--"
            btnToggle.text = "启动"
            btnToggle.backgroundTintList = ColorStateList.valueOf(getColor(R.color.primary))
            webView.visibility = View.GONE
            placeholder.visibility = View.VISIBLE
        }
    }

    private fun tryDiscoverServer() {
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
                    tvStatusText.setTextColor(getColor(R.color.success))
                    statusDot.setBackgroundResource(R.drawable.status_dot_running)
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
