//! Conta local: a chave de identidade guardada num "cofre" encriptado com a palavra-passe.
//!
//! - Criar conta gera 128 bits aleatórios, mostrados como 12 palavras de recuperação.
//!   A chave de identidade é derivada dessas palavras, por isso as 12 palavras recuperam
//!   sempre a mesma identidade.
//! - O cofre (ficheiro `conta.cofre`) tem a semente encriptada com XChaCha20-Poly1305,
//!   usando uma chave derivada da palavra-passe com Argon2id.
//! - O cofre pode ser copiado tal como está para o Google Drive: sem a palavra-passe
//!   ninguém o consegue abrir.

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow, bail, ensure};
use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce, aead::Aead};
use iroh::SecretKey;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, Zeroizing};

use crate::identity::Identity;

const VAULT_FILE: &str = "conta.cofre";
const VAULT_VERSION: u8 = 1;
const MIN_PASSWORD_CHARS: usize = 8;
const IDENTITY_CONTEXT: &str = "tasca 2026 identidade v1";

/// Custo do Argon2id: 64 MiB de memória, 3 passagens. Uns 0,3 s num PC normal.
const ARGON_M_KIB: u32 = 64 * 1024;
const ARGON_T: u32 = 3;
const ARGON_P: u32 = 1;

/// Conteúdo do ficheiro `conta.cofre`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Vault {
    pub version: u8,
    /// Nome mostrado antes de desbloquear ("Olá, Tiago").
    pub display_name: String,
    /// Chave pública, para mostrar e confirmar que o cofre é o certo.
    pub public_key: [u8; 32],
    pub kdf: KdfParams,
    pub salt: [u8; 16],
    pub nonce: [u8; 24],
    /// Semente de 16 bytes encriptada (+16 bytes de autenticação).
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KdfParams {
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

/// Uma conta desbloqueada.
pub struct Account {
    pub display_name: String,
    pub identity: Identity,
    seed: Zeroizing<[u8; 16]>,
}

impl std::fmt::Debug for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Account")
            .field("display_name", &self.display_name)
            .field("public_key", &self.identity.public())
            .finish_non_exhaustive()
    }
}

/// As 12 palavras de recuperação. Mostrar ao utilizador uma vez e pedir que as guarde.
pub struct RecoveryPhrase(Zeroizing<String>);

impl RecoveryPhrase {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn words(&self) -> Vec<&str> {
        self.0.split(' ').collect()
    }
}

impl Account {
    /// Cria uma conta nova. Devolve a conta e as 12 palavras de recuperação.
    pub fn create(display_name: &str, password: &str) -> Result<(Self, RecoveryPhrase)> {
        check_display_name(display_name)?;
        check_password(password)?;
        let mut seed = Zeroizing::new([0u8; 16]);
        getrandom::fill(seed.as_mut()).map_err(|e| anyhow!("gerador aleatório: {e}"))?;
        let phrase = phrase_from_seed(&seed)?;
        Ok((Self::from_seed(display_name.trim(), seed), phrase))
    }

    /// Recupera a conta a partir das 12 palavras (por exemplo, num PC novo).
    pub fn recover(phrase: &str, display_name: &str) -> Result<Self> {
        check_display_name(display_name)?;
        let normalized = phrase
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        let mnemonic = bip39::Mnemonic::parse_normalized(&normalized)
            .map_err(|e| anyhow!("palavras de recuperação inválidas: {e}"))?;
        let entropy = Zeroizing::new(mnemonic.to_entropy());
        ensure!(entropy.len() == 16, "são precisas exatamente 12 palavras");
        let mut seed = Zeroizing::new([0u8; 16]);
        seed.copy_from_slice(&entropy);
        Ok(Self::from_seed(display_name.trim(), seed))
    }

    fn from_seed(display_name: &str, seed: Zeroizing<[u8; 16]>) -> Self {
        let mut key = blake3::derive_key(IDENTITY_CONTEXT, seed.as_ref());
        let identity = Identity::from_secret(SecretKey::from_bytes(&key));
        key.zeroize();
        Self {
            display_name: display_name.to_string(),
            identity,
            seed,
        }
    }

    /// As palavras de recuperação desta conta (para as voltar a mostrar nas definições).
    pub fn recovery_phrase(&self) -> Result<RecoveryPhrase> {
        phrase_from_seed(&self.seed)
    }

    /// Fecha a conta num cofre encriptado com a palavra-passe.
    pub fn seal(&self, password: &str) -> Result<Vault> {
        check_password(password)?;
        let kdf = KdfParams {
            m_kib: ARGON_M_KIB,
            t: ARGON_T,
            p: ARGON_P,
        };
        let mut salt = [0u8; 16];
        let mut nonce = [0u8; 24];
        getrandom::fill(&mut salt).map_err(|e| anyhow!("gerador aleatório: {e}"))?;
        getrandom::fill(&mut nonce).map_err(|e| anyhow!("gerador aleatório: {e}"))?;
        let key = derive_key(password, &salt, kdf)?;
        let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|_| anyhow!("chave inválida"))?;
        let ciphertext = cipher
            .encrypt(&XNonce::from(nonce), self.seed.as_ref())
            .map_err(|_| anyhow!("falha ao encriptar"))?;
        Ok(Vault {
            version: VAULT_VERSION,
            display_name: self.display_name.clone(),
            public_key: *self.identity.public().as_bytes(),
            kdf,
            salt,
            nonce,
            ciphertext,
        })
    }
}

impl Vault {
    /// Abre o cofre com a palavra-passe.
    pub fn open(&self, password: &str) -> Result<Account> {
        ensure!(
            self.version == VAULT_VERSION,
            "versão do cofre desconhecida ({})",
            self.version
        );
        // Limites para não deixar um cofre malicioso pedir memória absurda.
        ensure!(
            self.kdf.m_kib <= 1024 * 1024 && self.kdf.t <= 16 && self.kdf.p <= 8,
            "parâmetros do cofre inválidos"
        );
        let key = derive_key(password, &self.salt, self.kdf)?;
        let cipher = XChaCha20Poly1305::new_from_slice(key.as_ref())
            .map_err(|_| anyhow!("chave inválida"))?;
        let plain = Zeroizing::new(
            cipher
                .decrypt(&XNonce::from(self.nonce), self.ciphertext.as_slice())
                .map_err(|_| anyhow!("palavra-passe errada"))?,
        );
        ensure!(plain.len() == 16, "cofre corrompido");
        let mut seed = Zeroizing::new([0u8; 16]);
        seed.copy_from_slice(&plain);
        let account = Account::from_seed(&self.display_name, seed);
        ensure!(
            account.identity.public().as_bytes() == &self.public_key,
            "cofre corrompido: a chave não corresponde"
        );
        Ok(account)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        postcard::to_stdvec(self).expect("serializar cofre")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        postcard::from_bytes(bytes).context("ficheiro de conta inválido")
    }

    pub fn public_key(&self) -> Result<iroh::PublicKey> {
        iroh::PublicKey::from_bytes(&self.public_key).map_err(|e| anyhow!("chave inválida: {e}"))
    }
}

/// Guarda e lê o cofre na pasta de dados da app.
#[derive(Clone, Debug)]
pub struct AccountStore {
    dir: PathBuf,
}

impl AccountStore {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            dir: data_dir.into(),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.dir.join(VAULT_FILE)
    }

    pub fn exists(&self) -> bool {
        self.path().exists()
    }

    pub fn load(&self) -> Result<Option<Vault>> {
        match std::fs::read(self.path()) {
            Ok(bytes) => Ok(Some(Vault::from_bytes(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e).context("ler o ficheiro da conta"),
        }
    }

    /// Grava o cofre de forma atómica (escreve num ficheiro temporário e troca).
    pub fn save(&self, vault: &Vault) -> Result<()> {
        std::fs::create_dir_all(&self.dir)?;
        let tmp = self.dir.join(format!("{VAULT_FILE}.tmp"));
        std::fs::write(&tmp, vault.to_bytes())?;
        std::fs::rename(&tmp, self.path())?;
        Ok(())
    }

    /// Cria a conta e grava o cofre. Recusa se já existir uma conta neste PC.
    pub fn create(&self, display_name: &str, password: &str) -> Result<(Account, RecoveryPhrase)> {
        if self.exists() {
            bail!("já existe uma conta neste PC");
        }
        let (account, phrase) = Account::create(display_name, password)?;
        self.save(&account.seal(password)?)?;
        Ok((account, phrase))
    }

    /// Desbloqueia a conta guardada neste PC.
    pub fn unlock(&self, password: &str) -> Result<Account> {
        let vault = self
            .load()?
            .ok_or_else(|| anyhow!("ainda não há conta neste PC"))?;
        vault.open(password)
    }

    /// Recupera com as 12 palavras e grava um cofre novo com a palavra-passe nova.
    /// Substitui a conta que existir neste PC.
    pub fn recover(&self, phrase: &str, display_name: &str, new_password: &str) -> Result<Account> {
        let account = Account::recover(phrase, display_name)?;
        self.save(&account.seal(new_password)?)?;
        Ok(account)
    }

    /// Instala um cofre vindo de fora (Google Drive), depois de confirmar a palavra-passe.
    pub fn import(&self, vault: &Vault, password: &str) -> Result<Account> {
        let account = vault.open(password)?;
        self.save(vault)?;
        Ok(account)
    }

    pub fn change_password(&self, old: &str, new: &str) -> Result<Vault> {
        let account = self.unlock(old)?;
        let vault = account.seal(new)?;
        self.save(&vault)?;
        Ok(vault)
    }
}

fn derive_key(password: &str, salt: &[u8], kdf: KdfParams) -> Result<Zeroizing<[u8; 32]>> {
    let params = argon2::Params::new(kdf.m_kib, kdf.t, kdf.p, Some(32))
        .map_err(|e| anyhow!("parâmetros Argon2: {e}"))?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = Zeroizing::new([0u8; 32]);
    argon
        .hash_password_into(password.as_bytes(), salt, out.as_mut())
        .map_err(|e| anyhow!("Argon2: {e}"))?;
    Ok(out)
}

fn phrase_from_seed(seed: &[u8; 16]) -> Result<RecoveryPhrase> {
    let mnemonic = bip39::Mnemonic::from_entropy(seed).map_err(|e| anyhow!("{e}"))?;
    Ok(RecoveryPhrase(Zeroizing::new(
        mnemonic.words().collect::<Vec<_>>().join(" "),
    )))
}

fn check_password(password: &str) -> Result<()> {
    ensure!(
        password.chars().count() >= MIN_PASSWORD_CHARS,
        "a palavra-passe tem de ter pelo menos {MIN_PASSWORD_CHARS} caracteres"
    );
    Ok(())
}

fn check_display_name(name: &str) -> Result<()> {
    let n = name.trim().chars().count();
    ensure!(
        n > 0 && n <= 32,
        "o nome tem de ter entre 1 e 32 caracteres"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PW: &str = "palavra-passe-boa";

    #[test]
    fn criar_fechar_e_abrir() {
        let (conta, frase) = Account::create("Tiago", PW).unwrap();
        assert_eq!(frase.words().len(), 12);
        let cofre = conta.seal(PW).unwrap();
        let bytes = cofre.to_bytes();
        let aberta = Vault::from_bytes(&bytes).unwrap().open(PW).unwrap();
        assert_eq!(aberta.identity.public(), conta.identity.public());
        assert_eq!(aberta.display_name, "Tiago");
    }

    #[test]
    fn palavra_passe_errada_e_recusada() {
        let (conta, _) = Account::create("Tiago", PW).unwrap();
        let cofre = conta.seal(PW).unwrap();
        let err = cofre.open("outra-palavra-passe").unwrap_err();
        assert!(err.to_string().contains("errada"));
    }

    #[test]
    fn cofre_adulterado_e_recusado() {
        let (conta, _) = Account::create("Tiago", PW).unwrap();
        let mut cofre = conta.seal(PW).unwrap();
        cofre.ciphertext[0] ^= 1;
        assert!(cofre.open(PW).is_err());
        let mut cofre = conta.seal(PW).unwrap();
        cofre.public_key[0] ^= 1;
        assert!(cofre.open(PW).is_err());
    }

    #[test]
    fn palavras_recuperam_a_mesma_identidade() {
        let (conta, frase) = Account::create("Tiago", PW).unwrap();
        // Maiúsculas e espaços a mais não interessam.
        let escrita = format!("  {}  ", frase.as_str().to_uppercase().replace(' ', "   "));
        let rec = Account::recover(&escrita, "Tiago").unwrap();
        assert_eq!(rec.identity.public(), conta.identity.public());
        assert!(Account::recover("isto nao sao palavras validas", "Tiago").is_err());
    }

    #[test]
    fn palavra_passe_curta_e_recusada() {
        assert!(Account::create("Tiago", "curta").is_err());
    }

    #[test]
    fn guardar_no_disco() {
        let dir = std::env::temp_dir().join(format!("tasca-teste-{}", hex::encode(rand16())));
        let store = AccountStore::new(&dir);
        assert!(store.load().unwrap().is_none());
        let (conta, frase) = store.create("Tiago", PW).unwrap();
        assert!(
            store.create("Outro", PW).is_err(),
            "não pode criar duas contas"
        );
        assert_eq!(
            store.unlock(PW).unwrap().identity.public(),
            conta.identity.public()
        );
        store.change_password(PW, "nova-palavra-passe").unwrap();
        assert!(store.unlock(PW).is_err());
        assert!(store.unlock("nova-palavra-passe").is_ok());
        let rec = store
            .recover(frase.as_str(), "Tiago", "terceira-palavra")
            .unwrap();
        assert_eq!(rec.identity.public(), conta.identity.public());
        std::fs::remove_dir_all(dir).ok();
    }

    fn rand16() -> [u8; 16] {
        let mut b = [0u8; 16];
        getrandom::fill(&mut b).unwrap();
        b
    }
}
