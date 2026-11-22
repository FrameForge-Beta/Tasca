package pt.tasca.app

import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import android.webkit.JavascriptInterface
import androidx.core.app.NotificationCompat

/**
 * Mantém a Tasca ligada com a app em segundo plano (serviço em primeiro plano, com uma
 * notificação discreta). Sem isto, o Android adormece a app e as mensagens só chegam
 * quando a voltas a abrir. Desliga-se em Definições > Notificações.
 */
class Ligado : Service() {
  override fun onBind(intent: Intent?): IBinder? = null

  override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
    val nm = getSystemService(NotificationManager::class.java)
    nm.createNotificationChannel(
      NotificationChannel(CANAL, "Tasca ligada em segundo plano", NotificationManager.IMPORTANCE_MIN).apply {
        setShowBadge(false)
        description = "Para as mensagens chegarem com a app fechada"
      }
    )
    val abrir = PendingIntent.getActivity(
      this, 0,
      Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP),
      PendingIntent.FLAG_IMMUTABLE
    )
    val n = NotificationCompat.Builder(this, CANAL)
      .setSmallIcon(R.mipmap.ic_launcher)
      .setContentTitle("Tasca ligada")
      .setContentText("A receber mensagens em segundo plano")
      .setOngoing(true)
      .setSilent(true)
      .setContentIntent(abrir)
      .setPriority(NotificationCompat.PRIORITY_MIN)
      .build()
    try {
      if (Build.VERSION.SDK_INT >= 34) startForeground(1, n, ServiceInfo.FOREGROUND_SERVICE_TYPE_REMOTE_MESSAGING)
      else startForeground(1, n)
    } catch (e: Exception) {
      stopSelf()
    }
    return START_STICKY
  }

  companion object {
    const val CANAL = "ligado"
    private const val PREFS = "tasca"
    private const val CHAVE = "segundo_plano"

    fun ligado(ctx: Context) = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(CHAVE, true)

    /** Liga o serviço se estiver ativo nas definições. */
    fun arrancar(ctx: Context) {
      if (!ligado(ctx)) return
      try {
        ctx.startForegroundService(Intent(ctx, Ligado::class.java))
      } catch (e: Exception) {
        // o Android pode recusar (por exemplo, a arrancar em segundo plano): fica para a próxima
      }
    }

    fun definir(ctx: Context, on: Boolean) {
      ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putBoolean(CHAVE, on).apply()
      if (on) arrancar(ctx) else ctx.stopService(Intent(ctx, Ligado::class.java))
    }
  }
}

/** Para a interface: `TascaServico.definir(true|false)` e `TascaServico.ligado()`. */
class ServicoJs(private val ctx: Context) {
  @JavascriptInterface
  fun definir(on: Boolean) = Ligado.definir(ctx, on)

  @JavascriptInterface
  fun ligado(): Boolean = Ligado.ligado(ctx)
}
