//! Corpos binários dos comandos (áudio, vídeo, ficheiros).
//!
//! No PC a interface manda os bytes crus. No Android o WebView não deixa ler o corpo dos
//! pedidos, por isso o Tauri manda tudo em JSON: a interface envia lá os bytes em base64
//! (ou, se não o fizer, chegam como lista de números). Aqui aceita-se qualquer das formas.

use std::borrow::Cow;

use base64::Engine;
use tauri::ipc::InvokeBody;

pub fn bytes(body: &InvokeBody) -> Option<Cow<'_, [u8]>> {
    match body {
        InvokeBody::Raw(b) => Some(Cow::Borrowed(b.as_slice())),
        InvokeBody::Json(serde_json::Value::String(s)) => {
            base64::engine::general_purpose::STANDARD.decode(s).ok().map(Cow::Owned)
        }
        InvokeBody::Json(serde_json::Value::Array(l)) => l
            .iter()
            .map(|v| v.as_u64().and_then(|n| u8::try_from(n).ok()))
            .collect::<Option<Vec<u8>>>()
            .map(Cow::Owned),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aceita_as_tres_formas() {
        let cru = InvokeBody::Raw(vec![1, 2, 255]);
        let b64 = InvokeBody::Json(serde_json::json!("AQL/"));
        let lista = InvokeBody::Json(serde_json::json!([1, 2, 255]));
        for b in [cru, b64, lista] {
            assert_eq!(bytes(&b).as_deref(), Some(&[1u8, 2, 255][..]));
        }
        assert!(bytes(&InvokeBody::Json(serde_json::json!([1, 300]))).is_none());
        assert!(bytes(&InvokeBody::Json(serde_json::json!({}))).is_none());
    }
}
