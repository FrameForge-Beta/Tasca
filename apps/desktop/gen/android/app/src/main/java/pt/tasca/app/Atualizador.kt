package pt.tasca.app

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.core.content.FileProvider
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

/**
 * Atualiza a app sem passar pelo browser: descarrega o APK mais recente do GitHub para a
 * cache da app e abre o instalador do Android (a pessoa só confirma "Instalar").
 *
 * A interface chama `TascaAndroid.atualizar()`; o progresso volta por
 * `window.tascaAtualizacao({ estado, pct })`, com estado = "a_transferir", "permissao",
 * "a_instalar" ou "erro".
 */
class Atualizador(private val activity: Activity, private val webView: WebView) {
  private val url = "https://github.com/RubenFernandes2004/Tasca/releases/latest/download/Tasca.apk"
  @Volatile private var aCorrer = false

  private fun avisar(estado: String, pct: Int = 0, erro: String = "") {
    val js = "window.tascaAtualizacao && window.tascaAtualizacao({estado:'$estado',pct:$pct,erro:${org.json.JSONObject.quote(erro)}})"
    activity.runOnUiThread { webView.evaluateJavascript(js, null) }
  }

  @JavascriptInterface
  fun atualizar() {
    if (aCorrer) return
    // Android 8+: a app tem de poder instalar apps (a pessoa autoriza uma vez)
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && !activity.packageManager.canRequestPackageInstalls()) {
      avisar("permissao")
      activity.runOnUiThread {
        activity.startActivity(
          Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + activity.packageName))
        )
      }
      return
    }
    aCorrer = true
    Thread {
      try {
        val pasta = File(activity.cacheDir, "atualizacao").apply { mkdirs() }
        val apk = File(pasta, "Tasca.apk")
        var ligacao = URL(url).openConnection() as HttpURLConnection
        ligacao.instanceFollowRedirects = true
        ligacao.connectTimeout = 20000
        ligacao.readTimeout = 30000
        ligacao.connect()
        if (ligacao.responseCode !in 200..299) throw Exception("o GitHub respondeu ${ligacao.responseCode}")
        val total = ligacao.contentLengthLong
        var feito = 0L
        var ultimo = -1
        ligacao.inputStream.use { ent ->
          apk.outputStream().use { sai ->
            val buf = ByteArray(64 * 1024)
            while (true) {
              val n = ent.read(buf)
              if (n < 0) break
              sai.write(buf, 0, n)
              feito += n
              val pct = if (total > 0) (feito * 100 / total).toInt() else 0
              if (pct != ultimo) { ultimo = pct; avisar("a_transferir", pct) }
            }
          }
        }
        ligacao.disconnect()
        avisar("a_instalar", 100)
        val uri = FileProvider.getUriForFile(activity, activity.packageName + ".fileprovider", apk)
        val instalar = Intent(Intent.ACTION_VIEW).apply {
          setDataAndType(uri, "application/vnd.android.package-archive")
          addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK)
        }
        activity.runOnUiThread { activity.startActivity(instalar) }
      } catch (e: Exception) {
        avisar("erro", 0, e.message ?: e.toString())
      } finally {
        aCorrer = false
      }
    }.start()
  }
}
