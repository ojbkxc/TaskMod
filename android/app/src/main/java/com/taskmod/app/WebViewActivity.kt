package com.taskmod.app

import android.annotation.SuppressLint
import android.os.Bundle
import android.view.View
import android.webkit.*
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.appcompat.app.AppCompatActivity
import com.google.android.material.button.MaterialButton

class WebViewActivity : AppCompatActivity() {

    private lateinit var webView: WebView
    private lateinit var progressBar: ProgressBar
    private lateinit var errorLayout: View
    private lateinit var tvErrorMessage: TextView
    private lateinit var btnRetry: MaterialButton

    private var currentUrl: String = ""

    @SuppressLint("SetJavaScriptEnabled")
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_webview)

        webView = findViewById(R.id.webview)
        progressBar = findViewById(R.id.progress_bar)
        errorLayout = findViewById(R.id.error_layout)
        tvErrorMessage = findViewById(R.id.tv_error_message)
        btnRetry = findViewById(R.id.btn_retry)

        currentUrl = intent.getStringExtra("url") ?: "http://127.0.0.1:${ConfigManager.getPort()}"

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
                val requestUrl = request?.url
                if (requestUrl != null) {
                    val host = requestUrl.host ?: ""
                    if (host in setOf("localhost", "127.0.0.1") || host.startsWith("10.") ||
                        host.startsWith("192.168.") || host.startsWith("172.16.") ||
                        host.startsWith("172.17.") || host.startsWith("172.18.") ||
                        host.startsWith("172.19.") || host.startsWith("172.20.") ||
                        host.startsWith("172.21.") || host.startsWith("172.22.") ||
                        host.startsWith("172.23.") || host.startsWith("172.24.") ||
                        host.startsWith("172.25.") || host.startsWith("172.26.") ||
                        host.startsWith("172.27.") || host.startsWith("172.28.") ||
                        host.startsWith("172.29.") || host.startsWith("172.30.") ||
                        host.startsWith("172.31.")) {
                        return false
                    }
                }
                val uri = request?.url ?: return true
                android.content.Intent(android.content.Intent.ACTION_VIEW, uri).let {
                    startActivity(it)
                }
                return true
            }

            override fun onReceivedError(view: WebView?, request: WebResourceRequest?, error: WebResourceError?) {
                super.onReceivedError(view, request, error)
                if (request?.isForMainFrame == true) {
                    showError("连接失败: ${error?.description ?: "未知错误"}")
                }
            }

            override fun onReceivedHttpError(view: WebView?, request: WebResourceRequest?, errorResponse: WebResourceResponse?) {
                super.onReceivedHttpError(view, request, errorResponse)
                if (request?.isForMainFrame == true) {
                    showError("服务器错误: ${errorResponse?.statusCode ?: "未知"}")
                }
            }
        }

        btnRetry.setOnClickListener {
            hideError()
            webView.loadUrl(currentUrl)
        }

        webView.loadUrl(currentUrl)
    }

    private fun showError(message: String) {
        webView.visibility = View.GONE
        errorLayout.visibility = View.VISIBLE
        tvErrorMessage.text = message
    }

    private fun hideError() {
        errorLayout.visibility = View.GONE
        webView.visibility = View.VISIBLE
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