package com.taskmod.app

import android.app.DownloadManager
import android.content.*
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.os.Environment
import android.os.Handler
import android.os.Looper
import android.view.View
import android.widget.*
import androidx.appcompat.app.AppCompatActivity
import androidx.core.content.FileProvider
import com.google.android.material.button.MaterialButton
import com.google.android.material.card.MaterialCardView
import com.google.android.material.dialog.MaterialAlertDialogBuilder
import com.google.gson.Gson
import com.google.gson.JsonObject
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

class MagiskGuideActivity : AppCompatActivity() {

    companion object {
        private const val API_URL = "https://api.github.com/repos/${TaskModApp.GITHUB_REPO}/releases/latest"
        private const val DOWNLOAD_DIR = "TaskMod"
    }

    private val handler = Handler(Looper.getMainLooper())

    // Views
    private lateinit var progressContainer: LinearLayout
    private lateinit var stepsContainer: LinearLayout
    private lateinit var tvCurrentStep: TextView
    private lateinit var progressBar: ProgressBar

    // Step views
    private lateinit var step1Card: MaterialCardView
    private lateinit var step1Status: ImageView
    private lateinit var step1Text: TextView

    private lateinit var step2Card: MaterialCardView
    private lateinit var step2Status: ImageView
    private lateinit var step2Text: TextView
    private lateinit var step2Version: TextView

    private lateinit var step3Card: MaterialCardView
    private lateinit var step3Status: ImageView
    private lateinit var step3Text: TextView
    private lateinit var btnDownload: MaterialButton

    private lateinit var step4Card: MaterialCardView
    private lateinit var step4Status: ImageView
    private lateinit var step4Text: TextView
    private lateinit var btnFlash: MaterialButton

    private lateinit var btnRetry: MaterialButton
    private lateinit var btnSkip: MaterialButton

    private var latestVersion: String = ""
    private var latestZipUrl: String = ""
    private var latestZipName: String = ""
    private var downloadedFile: File? = null
    private var downloadId: Long = -1

    private val downloadReceiver = object : BroadcastReceiver() {
        override fun onReceive(context: Context, intent: Intent) {
            if (intent.action == DownloadManager.ACTION_DOWNLOAD_COMPLETE) {
                val id = intent.getLongExtra(DownloadManager.EXTRA_DOWNLOAD_ID, -1)
                if (id == downloadId) {
                    onDownloadComplete()
                }
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_magisk_guide)

        initViews()
        startGuide()

        // 注册下载完成广播
        val filter = IntentFilter(DownloadManager.ACTION_DOWNLOAD_COMPLETE)
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            registerReceiver(downloadReceiver, filter, RECEIVER_NOT_EXPORTED)
        } else {
            registerReceiver(downloadReceiver, filter)
        }
    }

    private fun initViews() {
        progressContainer = findViewById(R.id.progress_container)
        stepsContainer = findViewById(R.id.steps_container)
        tvCurrentStep = findViewById(R.id.tv_current_step)
        progressBar = findViewById(R.id.progress_bar)

        step1Card = findViewById(R.id.step1_card)
        step1Status = findViewById(R.id.step1_status)
        step1Text = findViewById(R.id.step1_text)

        step2Card = findViewById(R.id.step2_card)
        step2Status = findViewById(R.id.step2_status)
        step2Text = findViewById(R.id.step2_text)
        step2Version = findViewById(R.id.step2_version)

        step3Card = findViewById(R.id.step3_card)
        step3Status = findViewById(R.id.step3_status)
        step3Text = findViewById(R.id.step3_text)
        btnDownload = findViewById(R.id.btn_download)

        step4Card = findViewById(R.id.step4_card)
        step4Status = findViewById(R.id.step4_status)
        step4Text = findViewById(R.id.step4_text)
        btnFlash = findViewById(R.id.btn_flash)

        btnRetry = findViewById(R.id.btn_retry)
        btnSkip = findViewById(R.id.btn_skip)

        btnDownload.setOnClickListener { downloadModule() }
        btnFlash.setOnClickListener { flashModule() }
        btnRetry.setOnClickListener { startGuide() }
        btnSkip.setOnClickListener { finish() }
    }

    private fun startGuide() {
        btnRetry.visibility = View.GONE
        stepsContainer.visibility = View.VISIBLE

        resetAllSteps()
        step1CheckMagisk()
    }

    private fun resetAllSteps() {
        step1Status.setImageResource(android.R.drawable.presence_invisible)
        step2Status.setImageResource(android.R.drawable.presence_invisible)
        step3Status.setImageResource(android.R.drawable.presence_invisible)
        step4Status.setImageResource(android.R.drawable.presence_invisible)
        step2Version.visibility = View.GONE
        btnDownload.visibility = View.GONE
        btnFlash.visibility = View.GONE
    }

    // Step 1: 检测 Magisk
    private fun step1CheckMagisk() {
        tvCurrentStep.text = "步骤 1/4：检测 Magisk 环境"
        step1Card.setCardBackgroundColor(getColor(R.color.surface_variant))
        step1Text.text = "正在检测 Magisk…"

        Thread {
            val hasMagisk = RootHelper.checkRoot().method == "magisk"
            handler.post {
                if (hasMagisk) {
                    step1Status.setImageResource(android.R.drawable.presence_online)
                    step1Text.text = "已检测到 Magisk"
                    step2FetchLatest()
                } else {
                    step1Status.setImageResource(android.R.drawable.presence_busy)
                    step1Text.text = "未检测到 Magisk"
                    showNoMagiskDialog()
                }
            }
        }.start()
    }

    // Step 2: 获取最新版本
    private fun step2FetchLatest() {
        tvCurrentStep.text = "步骤 2/4：获取最新版本信息"
        step2Card.setCardBackgroundColor(getColor(R.color.surface_variant))
        step2Text.text = "正在查询 GitHub Releases…"

        Thread {
            try {
                val url = URL(API_URL)
                val conn = url.openConnection() as HttpURLConnection
                conn.setRequestProperty("Accept", "application/vnd.github.v3+json")
                conn.connectTimeout = 10000
                conn.readTimeout = 10000

                if (conn.responseCode == 200) {
                    val response = conn.inputStream.bufferedReader().readText()
                    val json = Gson().fromJson(response, JsonObject::class.java)
                    val tagName = json.get("tag_name")?.asString ?: ""
                    latestVersion = tagName.removePrefix("v")

                    // 找到 Magisk 模块 zip 文件
                    val assets = json.getAsJsonArray("assets")
                    for (asset in assets) {
                        val assetObj = asset.asJsonObject
                        val name = assetObj.get("name")?.asString ?: ""
                        if (name.endsWith(".zip") && !name.contains("apk", ignoreCase = true)) {
                            latestZipUrl = assetObj.get("browser_download_url")?.asString ?: ""
                            latestZipName = name
                            break
                        }
                    }

                    // 如果没找到，构造默认 URL
                    if (latestZipUrl.isEmpty()) {
                        latestZipUrl = "https://github.com/${TaskModApp.GITHUB_REPO}/releases/download/$tagName/TaskMod-${latestVersion}.zip"
                        latestZipName = "TaskMod-${latestVersion}.zip"
                    }

                    conn.disconnect()

                    handler.post {
                        if (latestVersion.isNotEmpty()) {
                            step2Status.setImageResource(android.R.drawable.presence_online)
                            step2Text.text = "最新版本: v$latestVersion"
                            step2Version.text = latestZipName
                            step2Version.visibility = View.VISIBLE
                            step3Download()
                        } else {
                            step2Status.setImageResource(android.R.drawable.presence_busy)
                            step2Text.text = "获取版本信息失败"
                            btnRetry.visibility = View.VISIBLE
                        }
                    }
                } else {
                    conn.disconnect()
                    handler.post {
                        step2Status.setImageResource(android.R.drawable.presence_busy)
                        step2Text.text = "网络请求失败 (${conn.responseCode})"
                        btnRetry.visibility = View.VISIBLE
                    }
                }
            } catch (e: Exception) {
                handler.post {
                    step2Status.setImageResource(android.R.drawable.presence_busy)
                    step2Text.text = "网络错误: ${e.message}"
                    btnRetry.visibility = View.VISIBLE
                }
            }
        }.start()
    }

    // Step 3: 下载模块
    private fun step3Download() {
        tvCurrentStep.text = "步骤 3/4：下载 Magisk 模块"
        step3Card.setCardBackgroundColor(getColor(R.color.surface_variant))
        step3Text.text = "点击下方按钮下载"
        btnDownload.visibility = View.VISIBLE
        btnDownload.text = "下载 TaskMod v$latestVersion"
    }

    private fun downloadModule() {
        btnDownload.isEnabled = false
        btnDownload.text = "正在下载…"
        step3Text.text = "正在下载 $latestZipName …"

        try {
            val request = DownloadManager.Request(Uri.parse(latestZipUrl))
                .setTitle("TaskMod Magisk 模块")
                .setDescription("正在下载 TaskMod v$latestVersion")
                .setDestinationInExternalPublicDir(
                    Environment.DIRECTORY_DOWNLOADS,
                    "$DOWNLOAD_DIR/$latestZipName"
                )
                .setNotificationVisibility(DownloadManager.Request.VISIBILITY_VISIBLE_NOTIFY_COMPLETED)
                .setAllowedOverMetered(true)
                .setAllowedOverRoaming(true)

            val dm = getSystemService(Context.DOWNLOAD_SERVICE) as DownloadManager
            downloadId = dm.enqueue(request)

            // 保存下载信息
            getSharedPreferences("taskmod", MODE_PRIVATE).edit()
                .putLong("magisk_download_id", downloadId)
                .putString("magisk_zip_name", latestZipName)
                .apply()

        } catch (e: Exception) {
            btnDownload.isEnabled = true
            btnDownload.text = "重新下载"
            step3Text.text = "下载失败: ${e.message}"
        }
    }

    private fun onDownloadComplete() {
        val file = File(
            Environment.getExternalStoragePublicDirectory(Environment.DIRECTORY_DOWNLOADS),
            "$DOWNLOAD_DIR/$latestZipName"
        )

        if (file.exists()) {
            downloadedFile = file
            handler.post {
                step3Status.setImageResource(android.R.drawable.presence_online)
                step3Text.text = "下载完成: ${file.name}"
                btnDownload.visibility = View.GONE
                step4Flash()
            }
        } else {
            handler.post {
                step3Status.setImageResource(android.R.drawable.presence_busy)
                step3Text.text = "下载完成但文件未找到"
                btnDownload.isEnabled = true
                btnDownload.text = "重新下载"
            }
        }
    }

    // Step 4: 刷入模块
    private fun step4Flash() {
        tvCurrentStep.text = "步骤 4/4：刷入 Magisk 模块"
        step4Card.setCardBackgroundColor(getColor(R.color.surface_variant))
        step4Text.text = "请在 Magisk 中刷入下载的模块"
        btnFlash.visibility = View.VISIBLE
    }

    private fun flashModule() {
        val file = downloadedFile ?: return

        MaterialAlertDialogBuilder(this)
            .setTitle("刷入模块")
            .setMessage("""
                请按以下步骤操作：
                
                1. 打开 Magisk App
                2. 进入 "模块" 页面
                3. 点击 "从本地安装"
                4. 选择下载的文件：
                   ${file.absolutePath}
                5. 等待安装完成后重启设备
                
                重启后 TaskMod 服务将自动启动。
            """.trimIndent())
            .setPositiveButton("打开 Magisk") { _, _ ->
                openMagisk()
            }
            .setNegativeButton("打开文件管理器") { _, _ ->
                openFileManager(file)
            }
            .setNeutralButton("我已了解", null)
            .show()
    }

    private fun openMagisk() {
        try {
            val intent = packageManager.getLaunchIntentForPackage("com.topjohnwu.magisk")
            if (intent != null) {
                startActivity(intent)
            } else {
                Toast.makeText(this, "未安装 Magisk App", Toast.LENGTH_SHORT).show()
            }
        } catch (e: Exception) {
            Toast.makeText(this, "打开 Magisk 失败", Toast.LENGTH_SHORT).show()
        }
    }

    private fun openFileManager(file: File) {
        try {
            val uri = FileProvider.getUriForFile(this, "$packageName.fileprovider", file)
            val intent = Intent(Intent.ACTION_VIEW).apply {
                setDataAndType(uri, "application/zip")
                addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
            }
            startActivity(intent)
        } catch (e: Exception) {
            Toast.makeText(this, "请手动打开文件管理器找到: ${file.absolutePath}", Toast.LENGTH_LONG).show()
        }
    }

    private fun showNoMagiskDialog() {
        MaterialAlertDialogBuilder(this)
            .setTitle("需要 Magisk")
            .setMessage("""
                TaskMod 模块需要 Magisk 环境才能刷入。
                
                请先安装 Magisk：
                1. 解锁 Bootloader
                2. 刷入 Magisk
                3. 返回此页面重试
                
                如果您已有 Root 权限但未使用 Magisk，
                可以跳过此步骤，直接使用 APK 内置服务。
            """.trimIndent())
            .setPositiveButton("了解 Magisk") { _, _ ->
                val intent = Intent(Intent.ACTION_VIEW, Uri.parse("https://topjohnwu.github.io/Magisk/"))
                startActivity(intent)
            }
            .setNegativeButton("跳过，使用内置服务") { _, _ ->
                finish()
            }
            .setCancelable(false)
            .show()
    }

    override fun onDestroy() {
        try { unregisterReceiver(downloadReceiver) } catch (e: Exception) {}
        super.onDestroy()
    }
}
