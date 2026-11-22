package pt.tasca.app

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.pm.PackageInstaller
import android.net.Uri
import android.os.Build
import android.provider.Settings
import android.webkit.JavascriptInterface
import android.webkit.WebView
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

/**
 * Atualiza a app sozinha.
 *
 * O APK mais recente é descarregado em segundo plano para a cache da app e instalado
 * com o PackageInstaller do Android. Da primeira vez o Android pede confirmação
 * ("Atualizar esta app?"). A partir daí a Tasca passa a ser a "instaladora" de si
 * própria e, no Android 12 ou mais recente, as atualizações seguintes instalam-se sem
 * perguntar nada (permissão UPDATE_PACKAGES_WITHOUT_USER_ACTION). Para não fechar a app
 * a meio de uma conversa, a instalação silenciosa só acontece quando sais da app e não
 * estás numa chamada.
 *
 * Para a interface (`TascaAndroid`):
 * - `preparar()`: descarrega a versão nova sem instalar;
 * - `sozinha()`: true se a próxima atualização pode instalar-se sem perguntar;
 * - `instalarAoSair(on)`: instala quando a app for para segundo plano;
 * - `atualizar()`: descarrega (se preciso) e instala já.
 * O progresso volta por `window.tascaAtualizacao({ estado, pct, erro })`, com estado =
 * "a_transferir", "pronto", "permissao", "a_instalar" ou "erro".
 */
class Atualizador(private val activity: Activity, private val webView: WebView) {
  private val url = "https://github.com/RubenFernandes2004/Tasca/releases/latest/download/Tasca.apk"
  @Volatile private var aCorrer = false
  @Volatile private var pronto: File? = null
  @Volatile private var aoSair = false

  private fun avisar(estado: String, pct: Int = 0, erro: String = "") {
    val js = "window.tascaAtualizacao && window.tascaAtualizacao({estado:'$estado',pct:$pct,erro:${org.json.JSONObject.quote(erro)}})"
    activity.runOnUiThread { webView.evaluateJavascript(js, null) }
  }

  private fun podeInstalar(): Boolean =
    Build.VERSION.SDK_INT < Build.VERSION_CODES.O || activity.packageManager.canRequestPackageInstalls()

  private fun pedirPermissao() {
    avisar("permissao")
    activity.runOnUiThread {
      activity.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + activity.packageName)))
    }
  }

  /** A Tasca foi instalada por ela própria (a última atualização foi por aqui)? */
  @JavascriptInterface
  fun sozinha(): Boolean {
    if (Build.VERSION.SDK_INT < 31 || !podeInstalar()) return false
    return try {
      val info = activity.packageManager.getInstallSourceInfo(activity.packageName)
      info.installingPackageName == activity.packageName ||
        (Build.VERSION.SDK_INT >= 34 && info.updateOwnerPackageName == activity.packageName)
    } catch (e: Exception) { false }
  }

  @JavascriptInterface
  fun preparar() { transferir(instalarNoFim = false) }

  @JavascriptInterface
  fun atualizar() {
    if (!podeInstalar()) { pedirPermissao(); return }
    val f = pronto
    if (f != null && f.exists()) Thread { instalar(f) }.start() else transferir(instalarNoFim = true)
  }

  @JavascriptInterface
  fun instalarAoSair(on: Boolean) { aoSair = on }

  /** Chamado pela MainActivity quando a app vai para segundo plano. */
  fun saiu() {
    val f = pronto ?: return
    if (!aoSair || !f.exists() || !sozinha()) return
    aoSair = false
    Thread { instalar(f) }.start()
  }

  private fun transferir(instalarNoFim: Boolean) {
    if (aCorrer) return
    aCorrer = true
    Thread {
      try {
        val pasta = File(activity.cacheDir, "atualizacao").apply { mkdirs() }
        val apk = File(pasta, "Tasca.apk")
        val parcial = File(pasta, "Tasca.apk.parte")
        val ligacao = URL(url).openConnection() as HttpURLConnection
        ligacao.instanceFollowRedirects = true
        ligacao.connectTimeout = 20000
        ligacao.readTimeout = 30000
        ligacao.connect()
        if (ligacao.responseCode !in 200..299) throw Exception("o GitHub respondeu ${ligacao.responseCode}")
        val total = ligacao.contentLengthLong
        var feito = 0L
        var ultimo = -1
        ligacao.inputStream.use { ent ->
          parcial.outputStream().use { sai ->
            val buf = ByteArray(64 * 1024)
            while (true) {
              val n = ent.read(buf)
              if (n < 0) break
              sai.write(buf, 0, n)
              feito += n
              val pct = if (total > 0) (feito * 100 / total).toInt() else 0
              if (pct != ultimo) { ultimo = pct; if (instalarNoFim) avisar("a_transferir", pct) }
            }
          }
        }
        ligacao.disconnect()
        if (total > 0 && feito != total) throw Exception("a transferência ficou a meio")
        apk.delete(); parcial.renameTo(apk)
        pronto = apk
        avisar("pronto", 100)
        if (instalarNoFim) instalar(apk)
      } catch (e: Exception) {
        avisar("erro", 0, e.message ?: e.toString())
      } finally {
        aCorrer = false
      }
    }.start()
  }

  private fun instalar(apk: File) {
    try {
      avisar("a_instalar", 100)
      val pi = activity.packageManager.packageInstaller
      val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply {
        setAppPackageName(activity.packageName)
        if (Build.VERSION.SDK_INT >= 31) setRequireUserAction(PackageInstaller.SessionParams.USER_ACTION_NOT_REQUIRED)
      }
      val id = pi.createSession(params)
      pi.openSession(id).use { s ->
        s.openWrite("Tasca.apk", 0, apk.length()).use { sai -> apk.inputStream().use { it.copyTo(sai) }; s.fsync(sai) }
        val flags = PendingIntent.FLAG_UPDATE_CURRENT or (if (Build.VERSION.SDK_INT >= 31) PendingIntent.FLAG_MUTABLE else 0)
        val resposta = PendingIntent.getBroadcast(
          activity, id, Intent(activity, RespostaInstalacao::class.java).setPackage(activity.packageName), flags
        )
        s.commit(resposta.intentSender)
      }
    } catch (e: Exception) {
      avisar("erro", 0, e.message ?: e.toString())
    }
  }
}

/** Resposta do instalador: quando o Android precisa que confirmes, abre o pedido. */
class RespostaInstalacao : BroadcastReceiver() {
  override fun onReceive(ctx: Context, intent: Intent) {
    when (intent.getIntExtra(PackageInstaller.EXTRA_STATUS, PackageInstaller.STATUS_FAILURE)) {
      PackageInstaller.STATUS_PENDING_USER_ACTION -> {
        val pedir: Intent? = if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra(Intent.EXTRA_INTENT, Intent::class.java)
          else @Suppress("DEPRECATION") intent.getParcelableExtra(Intent.EXTRA_INTENT)
        pedir?.let { ctx.startActivity(it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
      }
      else -> {}
    }
  }
}
