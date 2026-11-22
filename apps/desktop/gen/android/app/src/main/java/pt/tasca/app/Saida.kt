package pt.tasca.app

import android.content.Context
import android.media.AudioDeviceInfo
import android.media.AudioManager
import android.os.Build
import android.webkit.JavascriptInterface
import org.json.JSONArray
import org.json.JSONObject

/**
 * Por onde sai o som da chamada no telemóvel: altifalante, auscultador (encostado ao
 * ouvido), auriculares com fios ou Bluetooth. Para a interface:
 * `TascaAudio.saidas()` (JSON) e `TascaAudio.escolher(id)`.
 */
class Saida(ctx: Context) {
  private val am = ctx.getSystemService(Context.AUDIO_SERVICE) as AudioManager

  private fun tipo(d: AudioDeviceInfo): String? = when (d.type) {
    AudioDeviceInfo.TYPE_BUILTIN_SPEAKER -> "altifalante"
    AudioDeviceInfo.TYPE_BUILTIN_EARPIECE -> "telefone"
    AudioDeviceInfo.TYPE_WIRED_HEADSET, AudioDeviceInfo.TYPE_WIRED_HEADPHONES, AudioDeviceInfo.TYPE_USB_HEADSET -> "fios"
    AudioDeviceInfo.TYPE_BLUETOOTH_SCO -> "bluetooth"
    else -> if (Build.VERSION.SDK_INT >= 31 && d.type == AudioDeviceInfo.TYPE_BLE_HEADSET) "bluetooth" else null
  }

  @JavascriptInterface
  fun saidas(): String {
    val lista = JSONArray()
    val vistos = HashSet<String>()
    val atual = atual()
    val devs: List<AudioDeviceInfo> = if (Build.VERSION.SDK_INT >= 31) am.availableCommunicationDevices
      else am.getDevices(AudioManager.GET_DEVICES_OUTPUTS).toList()
    for (d in devs) {
      val t = tipo(d) ?: continue
      if (!vistos.add(t)) continue
      val nome = if (t == "bluetooth" && d.productName.isNotBlank()) d.productName.toString() else ""
      lista.put(JSONObject().put("id", t).put("nome", nome).put("on", t == atual))
    }
    return lista.toString()
  }

  @JavascriptInterface
  fun atual(): String {
    if (Build.VERSION.SDK_INT >= 31) return am.communicationDevice?.let { tipo(it) } ?: ""
    @Suppress("DEPRECATION")
    return when {
      am.isBluetoothScoOn -> "bluetooth"
      am.isSpeakerphoneOn -> "altifalante"
      else -> "telefone"
    }
  }

  @JavascriptInterface
  fun escolher(id: String): Boolean {
    return try {
      if (Build.VERSION.SDK_INT >= 31) {
        val d = am.availableCommunicationDevices.firstOrNull { tipo(it) == id } ?: return false
        am.setCommunicationDevice(d)
      } else {
        @Suppress("DEPRECATION")
        when (id) {
          "altifalante" -> { am.stopBluetoothSco(); am.isBluetoothScoOn = false; am.isSpeakerphoneOn = true }
          "bluetooth" -> { am.isSpeakerphoneOn = false; am.startBluetoothSco(); am.isBluetoothScoOn = true }
          else -> { am.stopBluetoothSco(); am.isBluetoothScoOn = false; am.isSpeakerphoneOn = false }
        }
        true
      }
    } catch (e: Exception) { false }
  }
}
