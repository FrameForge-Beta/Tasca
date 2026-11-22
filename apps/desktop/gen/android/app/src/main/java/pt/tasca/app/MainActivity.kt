package pt.tasca.app

import android.content.Intent
import android.media.projection.MediaProjectionManager
import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  // Partilha de ecrã (ver Ecra.kt): o Android pergunta "Começar a gravar?" e devolve a autorização aqui.
  private var pedidoEcra = Triple(720, 30, 2_500_000)
  private val autorizarEcra = registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { r ->
    val dados: Intent? = r.data
    if (r.resultCode == RESULT_OK && dados != null) {
      val (altura, fps, bitrate) = pedidoEcra
      try { Ecra.iniciar(applicationContext, r.resultCode, dados, altura, fps, bitrate) }
      catch (e: Exception) { Ecra.avisar("erro", "O Android não deixou partilhar o ecrã.") }
    } else Ecra.avisar("cancelado", "")
  }

  fun pedirEcra(altura: Int, fps: Int, bitrate: Int) {
    pedidoEcra = Triple(altura, fps, bitrate)
    val mpm = getSystemService(MediaProjectionManager::class.java)
    try { autorizarEcra.launch(mpm.createScreenCaptureIntent()) }
    catch (e: Exception) { Ecra.avisar("erro", "Este telemóvel não deixa partilhar o ecrã.") }
  }

  // Atualizações dentro da app (ver Atualizador.kt).
  private var atualizador: Atualizador? = null

  // Ao sair da app, uma versão nova já descarregada instala-se sozinha (se der).
  override fun onStop() {
    super.onStop()
    atualizador?.saiu()
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    atualizador = Atualizador(this, webView).also { webView.addJavascriptInterface(it, "TascaAndroid") }
    webView.addJavascriptInterface(ServicoJs(applicationContext), "TascaServico")
    webView.addJavascriptInterface(EcraJs(this), "TascaEcra")
    webView.addJavascriptInterface(Saida(applicationContext), "TascaAudio")
    Ecra.web = webView
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Mensagens com a app em segundo plano (ver Ligado.kt).
    Ligado.arrancar(applicationContext)
    // A interface não fica por baixo da barra de estado, da barra de navegação nem do
    // teclado: o conteúdo encolhe para caber, e a caixa de escrever fica sempre à vista.
    val raiz = findViewById<View>(android.R.id.content)
    ViewCompat.setOnApplyWindowInsetsListener(raiz) { v, insets ->
      val m = insets.getInsets(
        WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout() or WindowInsetsCompat.Type.ime()
      )
      v.setPadding(m.left, m.top, m.right, m.bottom)
      WindowInsetsCompat.CONSUMED
    }
  }
}
