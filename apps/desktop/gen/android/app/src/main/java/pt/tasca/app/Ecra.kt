package pt.tasca.app

import android.app.Activity
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.MediaCodec
import android.media.MediaCodecInfo
import android.media.MediaFormat
import android.media.projection.MediaProjection
import android.media.projection.MediaProjectionManager
import android.os.Build
import android.os.Bundle
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.util.Base64
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.core.app.NotificationCompat

/**
 * Partilha do ecrã do telemóvel numa chamada.
 *
 * O WebView do Android não tem getDisplayMedia, por isso a captura é nativa: o Android
 * pergunta "Começar a gravar?", o ecrã vai para um codificador H.264 (MediaCodec) e cada
 * imagem codificada segue para a interface (`window.tascaEcra.imagem`), que a manda para a
 * chamada pelo mesmo caminho do vídeo do PC (`video_send`). O Android exige um serviço em
 * primeiro plano do tipo mediaProjection enquanto se partilha.
 */
class Ecra : Service() {
  private var projecao: MediaProjection? = null
  private var ecra: VirtualDisplay? = null
  private var codec: MediaCodec? = null
  @Volatile private var parado = false

  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    if (intent == null) { stopSelf(); return START_NOT_STICKY }
    val nm = getSystemService(NotificationManager::class.java)
    nm.createNotificationChannel(
      NotificationChannel(CANAL, "Partilha de ecrã", NotificationManager.IMPORTANCE_LOW).apply {
        description = "Aparece enquanto partilhas o ecrã numa chamada"
      }
    )
    val abrir = PendingIntent.getActivity(
      this, 1,
      Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
      PendingIntent.FLAG_IMMUTABLE
    )
    val n = NotificationCompat.Builder(this, CANAL)
      .setSmallIcon(R.mipmap.ic_launcher)
      .setContentTitle("A partilhar o ecrã")
      .setContentText("Toca para voltar à chamada")
      .setOngoing(true)
      .setSilent(true)
      .setContentIntent(abrir)
      .build()
    try {
      if (Build.VERSION.SDK_INT >= 29) startForeground(2, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PROJECTION)
      else startForeground(2, n)
    } catch (e: Exception) {
      avisar("erro", "O Android não deixou partilhar o ecrã.")
      stopSelf(); return START_NOT_STICKY
    }

    val codigo = intent.getIntExtra("codigo", Activity.RESULT_CANCELED)
    val dados: Intent? = if (Build.VERSION.SDK_INT >= 33) intent.getParcelableExtra("dados", Intent::class.java)
      else @Suppress("DEPRECATION") intent.getParcelableExtra("dados")
    val altura = intent.getIntExtra("altura", 720)
    val fps = intent.getIntExtra("fps", 30)
    val bitrate = intent.getIntExtra("bitrate", 2_500_000)
    try {
      comecar(codigo, dados!!, altura, fps, bitrate)
    } catch (e: Exception) {
      avisar("erro", "Não deu para partilhar o ecrã: ${e.message}")
      parar()
    }
    return START_NOT_STICKY
  }

  private fun comecar(codigo: Int, dados: Intent, altura: Int, fps: Int, bitrate: Int) {
    val mpm = getSystemService(MediaProjectionManager::class.java)
    val p = mpm.getMediaProjection(codigo, dados) ?: throw IllegalStateException("sem autorização")
    projecao = p
    // Obrigatório no Android 14 antes de criar o ecrã virtual. Também apanha o "Parar"
    // da barra de notificações do sistema.
    p.registerCallback(object : MediaProjection.Callback() {
      override fun onStop() { if (!parado) { avisar("parou", ""); parar() } }
    }, Handler(Looper.getMainLooper()))

    val m = resources.displayMetrics
    val (lw, lh) = m.widthPixels to m.heightPixels
    // a altura pedida aplica-se ao lado menor (vídeo ao alto ou deitado), sem aumentar
    val menor = minOf(lw, lh)
    val escala = minOf(1.0, altura.toDouble() / menor)
    val w = ((lw * escala).toInt() / 16) * 16
    val h = ((lh * escala).toInt() / 16) * 16

    val fmt = MediaFormat.createVideoFormat(MediaFormat.MIMETYPE_VIDEO_AVC, w, h).apply {
      setInteger(MediaFormat.KEY_COLOR_FORMAT, MediaCodecInfo.CodecCapabilities.COLOR_FormatSurface)
      setInteger(MediaFormat.KEY_BIT_RATE, bitrate)
      setInteger(MediaFormat.KEY_FRAME_RATE, fps)
      setInteger(MediaFormat.KEY_I_FRAME_INTERVAL, 2)
      // ecrã parado continua a mandar imagens, para quem entra a meio ver alguma coisa
      setLong(MediaFormat.KEY_REPEAT_PREVIOUS_FRAME_AFTER, 1_000_000L / fps * 3)
      // o descodificador dos outros abre "avc1.42E034" (baseline)
      setInteger(MediaFormat.KEY_PROFILE, MediaCodecInfo.CodecProfileLevel.AVCProfileBaseline)
    }
    val c = MediaCodec.createEncoderByType(MediaFormat.MIMETYPE_VIDEO_AVC)
    try {
      c.configure(fmt, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
    } catch (e: Exception) {
      // alguns codificadores não aceitam o perfil escolhido: vai com o que tiverem
      fmt.removeKey(MediaFormat.KEY_PROFILE)
      c.configure(fmt, null, null, MediaCodec.CONFIGURE_FLAG_ENCODE)
    }
    val superficie = c.createInputSurface()
    c.start()
    codec = c
    ecra = p.createVirtualDisplay("Tasca", w, h, m.densityDpi, DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR, superficie, null, null)
    avisar("comecou", "${w}x${h}")
    Thread({ drenar(c) }, "tasca-ecra").start()
  }

  /** Tira as imagens codificadas e manda-as para a interface. */
  private fun drenar(c: MediaCodec) {
    val info = MediaCodec.BufferInfo()
    var config = ByteArray(0) // SPS/PPS: vão à frente de cada imagem chave
    while (!parado) {
      val i = try { c.dequeueOutputBuffer(info, 100_000) } catch (e: Exception) { break }
      if (i < 0) continue
      val buf = c.getOutputBuffer(i)
      if (buf != null && info.size > 0) {
        buf.position(info.offset); buf.limit(info.offset + info.size)
        val bytes = ByteArray(info.size); buf.get(bytes)
        if (info.flags and MediaCodec.BUFFER_FLAG_CODEC_CONFIG != 0) {
          config = bytes
        } else {
          val chave = info.flags and MediaCodec.BUFFER_FLAG_KEY_FRAME != 0
          val img = if (chave) config + bytes else bytes
          entregar(img, chave)
        }
      }
      try { c.releaseOutputBuffer(i, false) } catch (e: Exception) { break }
    }
  }

  private fun parar() {
    parado = true
    atual = null
    try { ecra?.release() } catch (_: Exception) {}
    try { codec?.stop() } catch (_: Exception) {}
    try { codec?.release() } catch (_: Exception) {}
    try { projecao?.stop() } catch (_: Exception) {}
    ecra = null; codec = null; projecao = null
    stopForeground(STOP_FOREGROUND_REMOVE)
    stopSelf()
  }

  override fun onCreate() { super.onCreate(); atual = this }
  override fun onDestroy() { if (!parado) parar(); super.onDestroy() }

  /** Pede uma imagem chave já (alguém entrou na chamada ou perdeu imagens). */
  fun pedirChave() {
    try { codec?.setParameters(Bundle().apply { putInt(MediaCodec.PARAMETER_KEY_REQUEST_SYNC_FRAME, 0) }) } catch (_: Exception) {}
  }

  companion object {
    const val CANAL = "ecra"
    @Volatile var atual: Ecra? = null
    @Volatile var web: WebView? = null
    @Volatile private var emVoo = 0

    private fun js(code: String) {
      val w = web ?: return
      w.post { w.evaluateJavascript(code, null) }
    }

    fun avisar(estado: String, texto: String) {
      val t = texto.replace("\\", "\\\\").replace("'", "\\'")
      js("window.tascaEcra && tascaEcra.estado('$estado', '$t')")
    }

    private fun entregar(img: ByteArray, chave: Boolean) {
      // interface atrasada: deita fora imagens normais (a próxima chave recupera)
      if (emVoo > 6 && !chave) { atual?.pedirChave(); return }
      val w = web ?: return
      emVoo++
      val b64 = Base64.encodeToString(img, Base64.NO_WRAP)
      w.post {
        w.evaluateJavascript("window.tascaEcra && tascaEcra.imagem('$b64', ${if (chave) 1 else 0})") { emVoo-- }
      }
    }

    fun iniciar(ctx: Context, codigo: Int, dados: Intent, altura: Int, fps: Int, bitrate: Int) {
      val i = Intent(ctx, Ecra::class.java)
        .putExtra("codigo", codigo).putExtra("dados", dados)
        .putExtra("altura", altura).putExtra("fps", fps).putExtra("bitrate", bitrate)
      ctx.startForegroundService(i)
    }

    fun parar(ctx: Context) {
      val a = atual
      if (a != null) a.parar() else ctx.stopService(Intent(ctx, Ecra::class.java))
    }
  }
}

/** Para a interface: `TascaEcra.iniciar(altura, fps, bitrate)`, `parar()`, `chave()`. */
class EcraJs(private val act: MainActivity) {
  @JavascriptInterface
  fun iniciar(altura: Int, fps: Int, bitrate: Int) {
    act.runOnUiThread { act.pedirEcra(altura, fps, bitrate) }
  }

  @JavascriptInterface
  fun parar() { act.runOnUiThread { Ecra.parar(act) } }

  @JavascriptInterface
  fun chave() { Ecra.atual?.pedirChave() }
}
