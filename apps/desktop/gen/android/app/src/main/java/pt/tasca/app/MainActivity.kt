package pt.tasca.app

import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  // Atualizações dentro da app (ver Atualizador.kt).
  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    webView.addJavascriptInterface(Atualizador(this, webView), "TascaAndroid")
    webView.addJavascriptInterface(ServicoJs(applicationContext), "TascaServico")
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
