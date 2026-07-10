package com.taskmod.app

import android.annotation.SuppressLint
import android.os.Bundle
import android.view.View
import android.webkit.*
import android.widget.ProgressBar
import androidx.appcompat.app.AppCompatActivity

class WebViewActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_webview)

        webView = findViewById(R.id.webview)
        progressBar = findViewById(R.id.progress_bar)

        val url = intent.getStringExtra("url") ?: "http://127.0.0.1:${ConfigManager.getPort()}"

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
            // 本地服务器不使用缓存，确保每次加载最新UI
            cacheMode = WebSettings.LOAD_NO_CACHE
            // 设置 User-Agent 便于服务端识别
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
                // 允许 localhost 和 127.0.0.1 的请求
                if (requestUrl.contains("localhost") || requestUrl.contains("127.0.0.1")) {
                    return false
                }
                // 其他链接用外部浏览器打开
                android.content.Intent(android.content.Intent.ACTION_VIEW, android.net.Uri.parse(requestUrl)).let {
                    startActivity(it)
                }
                return true
            }
        }

        webView.loadUrl(url)
    }

    override fun onBackPressed() {
        if (webView.canGoBack()) {
            webView.goBack()
        } else {
            super.onBackPressed()
        }
    }

    private fun getVersionName(): String {
        return try {
            packageManager.getPackageInfo(packageName, 0).versionName ?: "1.0.0"
        } catch (e: Exception) {
            "1.0.0"
        }
    }
}
