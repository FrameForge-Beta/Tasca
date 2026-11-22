#!/usr/bin/env python3
"""Gera e assina o catálogo da loja de plugins.

Lê catalogo.base.json, junta a cada plugin o SHA-256 do ficheiro (a app
confirma-o antes de instalar) e escreve catalogo.json e catalogo.json.sig (assinatura
Ed25519 da equipa, em hexadecimal).

Uso: TASCA_CHAVE_LOJA=<chave privada em hexadecimal> python3 plugins/assinar.py
A chave privada nunca vai para o repositório.
"""
import hashlib, json, os, sys
from pathlib import Path
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

aqui = Path(__file__).resolve().parent
chave = os.environ.get("TASCA_CHAVE_LOJA", "").strip()
if not chave:
    sys.exit("Falta TASCA_CHAVE_LOJA (chave privada da loja, em hexadecimal).")
base = json.loads((aqui / "catalogo.base.json").read_text(encoding="utf-8"))
for p in base["plugins"]:
    dados = (aqui / p["file"]).read_bytes()
    p["sha256"] = hashlib.sha256(dados).hexdigest()
    p["size"] = len(dados)
texto = json.dumps(base, ensure_ascii=False, indent=2).encode("utf-8")
(aqui / "catalogo.json").write_bytes(texto)
sig = Ed25519PrivateKey.from_private_bytes(bytes.fromhex(chave)).sign(texto)
(aqui / "catalogo.json.sig").write_text(sig.hex() + "\n")
print("Catálogo assinado:", len(base["plugins"]), "plugins")
