use std::fs;
use std::path::Path;

use chrono::Utc;
use maze_crypto::{self, DerivedKey, EncryptedBlob, KdfParams};
use rand::RngCore;
use ssh_key::{Algorithm, HashAlg, LineEnding, PrivateKey};
use uuid::Uuid;
use zeroize::Zeroize;

use crate::error::VaultError;
use crate::types::*;

const VAULT_META_FILE: &str = "vault-meta.json";
const KEYS_DIR: &str = "keys";
const VAULT_VERSION: u32 = 1;

pub struct SshKeyVault;

impl SshKeyVault {
    // ── Lifecycle ────────────────────────────────────────────────

    /// Initialize a new vault: generate random VEK, derive VMK from
    /// passphrase, encrypt VEK under VMK, write vault-meta.json.
    pub fn init(passphrase: &str, vault_dir: &Path) -> Result<(), VaultError> {
        let meta_path = vault_dir.join(VAULT_META_FILE);
        if meta_path.exists() {
            return Err(VaultError::AlreadyInitialized);
        }

        // Ensure directories exist
        fs::create_dir_all(vault_dir.join(KEYS_DIR))?;

        // Generate random VEK
        let mut vek = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut vek);

        // Derive VMK from passphrase
        let kdf_params = KdfParams::default();
        let vmk = derive_vmk(passphrase, &kdf_params)?;

        // Encrypt VEK under VMK
        let encrypted_vek = maze_crypto::encrypt(&vek, vmk.as_bytes())?;

        // Zeroize raw VEK
        vek.zeroize();

        let meta = VaultMeta {
            version: VAULT_VERSION,
            kdf_params,
            encrypted_vek,
            keys: Vec::new(),
        };

        save_meta(&meta, vault_dir)?;
        Ok(())
    }

    /// Check whether vault-meta.json exists.
    pub fn is_initialized(vault_dir: &Path) -> bool {
        vault_dir.join(VAULT_META_FILE).exists()
    }

    /// Unlock: derive VMK from passphrase, decrypt VEK, return session.
    pub fn unlock(passphrase: &str, vault_dir: &Path) -> Result<VaultSession, VaultError> {
        let meta = load_meta(vault_dir)?;
        let vmk = derive_vmk(passphrase, &meta.kdf_params)?;
        let vek = decrypt_vek(&vmk, &meta.encrypted_vek)?;
        Ok(VaultSession::new(vek))
    }

    /// Re-encrypt VEK under a new passphrase. Individual key files unchanged.
    pub fn change_passphrase(
        old_passphrase: &str,
        new_passphrase: &str,
        vault_dir: &Path,
    ) -> Result<(), VaultError> {
        let mut meta = load_meta(vault_dir)?;

        // Decrypt VEK with old passphrase
        let old_vmk = derive_vmk(old_passphrase, &meta.kdf_params)?;
        let vek = decrypt_vek(&old_vmk, &meta.encrypted_vek)?;

        // New KDF params with fresh salt
        let new_kdf_params = KdfParams::default();
        let new_vmk = derive_vmk(new_passphrase, &new_kdf_params)?;

        // Re-encrypt VEK under new VMK
        let new_encrypted_vek = maze_crypto::encrypt(&vek, new_vmk.as_bytes())?;

        meta.kdf_params = new_kdf_params;
        meta.encrypted_vek = new_encrypted_vek;
        save_meta(&meta, vault_dir)?;
        Ok(())
    }

    // ── Key CRUD ────────────────────────────────────────────────

    /// Generate a new SSH key pair, encrypt private key, store in vault.
    pub fn generate_key(
        session: &VaultSession,
        input: GenerateKeyInput,
        vault_dir: &Path,
    ) -> Result<SshKeyItem, VaultError> {
        let mut meta = load_meta(vault_dir)?;

        // Check duplicate name
        if meta.keys.iter().any(|k| k.name == input.name) {
            return Err(VaultError::DuplicateKeyName(input.name));
        }

        // Generate SSH key
        let private_key = generate_ssh_key(input.algorithm)?;
        let comment = input.comment.clone().unwrap_or_default();

        // Extract metadata
        let fingerprint = private_key
            .fingerprint(HashAlg::Sha256)
            .to_string();
        let public_key_openssh = if comment.is_empty() {
            private_key.public_key().to_openssh()
                .map_err(|e| VaultError::KeyGenError(e.to_string()))?
        } else {
            format!(
                "{} {}",
                private_key.public_key().to_openssh()
                    .map_err(|e| VaultError::KeyGenError(e.to_string()))?,
                comment
            )
        };

        // Serialize private key to OpenSSH PEM
        let mut private_pem = private_key
            .to_openssh(LineEnding::LF)
            .map_err(|e| VaultError::KeyGenError(e.to_string()))?
            .to_string();

        // Encrypt private key PEM under VEK
        let encrypted = maze_crypto::encrypt(private_pem.as_bytes(), session.vek())?;
        private_pem.zeroize();

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Write encrypted key file
        let key_path = vault_dir.join(KEYS_DIR).join(format!("{id}.enc"));
        let key_json = serde_json::to_string_pretty(&encrypted)?;
        atomic_write(&key_path, key_json.as_bytes())?;

        let item = SshKeyItem {
            id,
            name: input.name,
            algorithm: input.algorithm,
            fingerprint,
            public_key_openssh,
            state: KeyState::Active,
            export_policy: input.export_policy.unwrap_or_default(),
            comment,
            allowed_hosts: input.allowed_hosts,
            created_at: now,
            updated_at: now,
        };

        meta.keys.push(item.clone());
        save_meta(&meta, vault_dir)?;

        Ok(item)
    }

    /// Import an existing private key PEM into the vault.
    pub fn import_key(
        session: &VaultSession,
        input: ImportKeyInput,
        vault_dir: &Path,
    ) -> Result<SshKeyItem, VaultError> {
        let mut meta = load_meta(vault_dir)?;

        if meta.keys.iter().any(|k| k.name == input.name) {
            return Err(VaultError::DuplicateKeyName(input.name));
        }

        // Parse private key
        let private_key = parse_private_key(&input.private_key_pem, input.source_passphrase.as_deref())?;

        let comment = input.comment.clone().unwrap_or_default();
        let fingerprint = private_key.fingerprint(HashAlg::Sha256).to_string();

        let algorithm = match private_key.algorithm() {
            Algorithm::Ed25519 => KeyAlgorithm::Ed25519,
            Algorithm::Rsa { .. } => KeyAlgorithm::Rsa4096,
            other => return Err(VaultError::KeyParseError(format!("unsupported algorithm: {other:?}"))),
        };

        let public_key_openssh = if comment.is_empty() {
            private_key.public_key().to_openssh()
                .map_err(|e| VaultError::KeyParseError(e.to_string()))?
        } else {
            format!(
                "{} {}",
                private_key.public_key().to_openssh()
                    .map_err(|e| VaultError::KeyParseError(e.to_string()))?,
                comment
            )
        };

        // Re-serialize to normalized PEM (unencrypted)
        let mut private_pem = private_key
            .to_openssh(LineEnding::LF)
            .map_err(|e| VaultError::KeyParseError(e.to_string()))?
            .to_string();

        // Encrypt under VEK
        let encrypted = maze_crypto::encrypt(private_pem.as_bytes(), session.vek())?;
        private_pem.zeroize();

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let key_path = vault_dir.join(KEYS_DIR).join(format!("{id}.enc"));
        let key_json = serde_json::to_string_pretty(&encrypted)?;
        atomic_write(&key_path, key_json.as_bytes())?;

        let item = SshKeyItem {
            id,
            name: input.name,
            algorithm,
            fingerprint,
            public_key_openssh,
            state: KeyState::Active,
            export_policy: input.export_policy.unwrap_or_default(),
            comment,
            allowed_hosts: Vec::new(),
            created_at: now,
            updated_at: now,
        };

        meta.keys.push(item.clone());
        save_meta(&meta, vault_dir)?;

        Ok(item)
    }

    /// List all keys (summary only).
    pub fn list_keys(vault_dir: &Path) -> Result<Vec<SshKeyItemSummary>, VaultError> {
        let meta = load_meta(vault_dir)?;
        Ok(meta.keys.iter().map(SshKeyItemSummary::from).collect())
    }

    /// Get full metadata for a single key.
    pub fn get_key(id: &str, vault_dir: &Path) -> Result<SshKeyItem, VaultError> {
        let meta = load_meta(vault_dir)?;
        meta.keys
            .into_iter()
            .find(|k| k.id == id)
            .ok_or_else(|| VaultError::KeyNotFound(id.to_string()))
    }

    /// Update mutable metadata fields.
    pub fn update_key(
        id: &str,
        input: UpdateKeyInput,
        vault_dir: &Path,
    ) -> Result<SshKeyItem, VaultError> {
        let mut meta = load_meta(vault_dir)?;

        // Check duplicate name before borrowing mutably
        if let Some(name) = &input.name {
            if meta.keys.iter().any(|k| k.id != id && k.name == *name) {
                return Err(VaultError::DuplicateKeyName(name.clone()));
            }
        }

        let key = meta
            .keys
            .iter_mut()
            .find(|k| k.id == id)
            .ok_or_else(|| VaultError::KeyNotFound(id.to_string()))?;

        if let Some(name) = input.name {
            key.name = name;
        }
        if let Some(comment) = input.comment {
            key.comment = comment;
        }
        if let Some(policy) = input.export_policy {
            key.export_policy = policy;
        }
        if let Some(hosts) = input.allowed_hosts {
            key.allowed_hosts = hosts;
        }
        key.updated_at = Utc::now();

        let updated = key.clone();
        save_meta(&meta, vault_dir)?;
        Ok(updated)
    }

    /// Permanently delete a key.
    pub fn delete_key(
        _session: &VaultSession,
        id: &str,
        vault_dir: &Path,
    ) -> Result<(), VaultError> {
        let mut meta = load_meta(vault_dir)?;
        let pos = meta
            .keys
            .iter()
            .position(|k| k.id == id)
            .ok_or_else(|| VaultError::KeyNotFound(id.to_string()))?;

        meta.keys.remove(pos);

        // Remove encrypted key file
        let key_path = vault_dir.join(KEYS_DIR).join(format!("{id}.enc"));
        if key_path.exists() {
            fs::remove_file(&key_path)?;
        }

        save_meta(&meta, vault_dir)?;
        Ok(())
    }

    /// Set key state to Archived.
    pub fn archive_key(id: &str, vault_dir: &Path) -> Result<(), VaultError> {
        let mut meta = load_meta(vault_dir)?;
        let key = meta
            .keys
            .iter_mut()
            .find(|k| k.id == id)
            .ok_or_else(|| VaultError::KeyNotFound(id.to_string()))?;

        key.state = KeyState::Archived;
        key.updated_at = Utc::now();
        save_meta(&meta, vault_dir)?;
        Ok(())
    }

    // ── Export ───────────────────────────────────────────────────

    /// Return the OpenSSH public key string.
    pub fn export_public_key(id: &str, vault_dir: &Path) -> Result<String, VaultError> {
        let meta = load_meta(vault_dir)?;
        let key = meta
            .keys
            .iter()
            .find(|k| k.id == id)
            .ok_or_else(|| VaultError::KeyNotFound(id.to_string()))?;

        Ok(key.public_key_openssh.clone())
    }

    /// Decrypt and return the private key PEM. Checks export policy.
    pub fn export_private_key(
        session: &VaultSession,
        id: &str,
        vault_dir: &Path,
    ) -> Result<String, VaultError> {
        let meta = load_meta(vault_dir)?;
        let key = meta
            .keys
            .iter()
            .find(|k| k.id == id)
            .ok_or_else(|| VaultError::KeyNotFound(id.to_string()))?;

        if !key.export_policy.allow_private_export {
            return Err(VaultError::ExportDenied);
        }

        let pem_bytes = decrypt_key_file(session.vek(), id, vault_dir)?;
        String::from_utf8(pem_bytes)
            .map_err(|e| VaultError::KeyParseError(format!("private key is not valid UTF-8: {e}")))
    }

    // ── Signing (M2) ────────────────────────────────────────────

    /// Decrypt private key, sign data, return signature bytes.
    pub fn sign(
        session: &VaultSession,
        id: &str,
        data: &[u8],
        vault_dir: &Path,
    ) -> Result<Vec<u8>, VaultError> {
        let mut pem_bytes = decrypt_key_file(session.vek(), id, vault_dir)?;
        let pem_str = std::str::from_utf8(&pem_bytes)
            .map_err(|e| VaultError::KeyParseError(e.to_string()))?;

        let private_key = PrivateKey::from_openssh(pem_str)
            .map_err(|e| VaultError::KeyParseError(e.to_string()))?;

        use signature::Signer;
        let signature = private_key
            .try_sign(data)
            .map_err(|e| VaultError::KeyGenError(format!("signing failed: {e}")))?;

        pem_bytes.zeroize();

        Ok(signature.as_bytes().to_vec())
    }
}

// ── Internal helpers ────────────────────────────────────────────

fn load_meta(vault_dir: &Path) -> Result<VaultMeta, VaultError> {
    let path = vault_dir.join(VAULT_META_FILE);
    if !path.exists() {
        return Err(VaultError::NotInitialized(
            vault_dir.display().to_string(),
        ));
    }
    let content = fs::read_to_string(&path)?;
    let meta: VaultMeta = serde_json::from_str(&content)?;
    Ok(meta)
}

fn save_meta(meta: &VaultMeta, vault_dir: &Path) -> Result<(), VaultError> {
    let path = vault_dir.join(VAULT_META_FILE);
    let content = serde_json::to_string_pretty(meta)?;
    atomic_write(&path, content.as_bytes())?;
    Ok(())
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), std::io::Error> {
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn derive_vmk(passphrase: &str, params: &KdfParams) -> Result<DerivedKey, VaultError> {
    maze_crypto::derive_key(passphrase, params).map_err(VaultError::from)
}

fn decrypt_vek(vmk: &DerivedKey, encrypted_vek: &EncryptedBlob) -> Result<[u8; 32], VaultError> {
    let vek_bytes = maze_crypto::decrypt(encrypted_vek, vmk.as_bytes())?;
    if vek_bytes.len() != 32 {
        return Err(VaultError::Crypto(maze_crypto::CryptoError::DecryptionError(
            format!("VEK has wrong length: {} (expected 32)", vek_bytes.len()),
        )));
    }
    let mut vek = [0u8; 32];
    vek.copy_from_slice(&vek_bytes);
    Ok(vek)
}

fn decrypt_key_file(vek: &[u8; 32], id: &str, vault_dir: &Path) -> Result<Vec<u8>, VaultError> {
    let key_path = vault_dir.join(KEYS_DIR).join(format!("{id}.enc"));
    if !key_path.exists() {
        return Err(VaultError::KeyNotFound(id.to_string()));
    }
    let content = fs::read_to_string(&key_path)?;
    let blob: EncryptedBlob = serde_json::from_str(&content)?;
    let plaintext = maze_crypto::decrypt(&blob, vek)?;
    Ok(plaintext)
}

fn generate_ssh_key(algorithm: KeyAlgorithm) -> Result<PrivateKey, VaultError> {
    let mut rng = rand::rngs::OsRng;
    match algorithm {
        KeyAlgorithm::Ed25519 => PrivateKey::random(&mut rng, Algorithm::Ed25519),
        KeyAlgorithm::Rsa4096 => {
            PrivateKey::random(&mut rng, Algorithm::Rsa { hash: Some(HashAlg::Sha512) })
        }
    }
    .map_err(|e| VaultError::KeyGenError(e.to_string()))
}

fn parse_private_key(pem: &str, passphrase: Option<&str>) -> Result<PrivateKey, VaultError> {
    match passphrase {
        Some(pass) => {
            let encrypted = ssh_key::private::PrivateKey::from_openssh(pem)
                .map_err(|e| VaultError::KeyParseError(e.to_string()))?;
            encrypted
                .decrypt(pass)
                .map_err(|e| VaultError::KeyParseError(format!("decryption failed: {e}")))
        }
        None => PrivateKey::from_openssh(pem)
            .map_err(|e| VaultError::KeyParseError(e.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn init_creates_vault() {
        let dir = setup();
        SshKeyVault::init("test_pass", dir.path()).unwrap();
        assert!(SshKeyVault::is_initialized(dir.path()));
        assert!(dir.path().join("vault-meta.json").exists());
        assert!(dir.path().join("keys").exists());
    }

    #[test]
    fn init_twice_fails() {
        let dir = setup();
        SshKeyVault::init("test_pass", dir.path()).unwrap();
        let result = SshKeyVault::init("test_pass", dir.path());
        assert!(matches!(result, Err(VaultError::AlreadyInitialized)));
    }

    #[test]
    fn unlock_correct_passphrase() {
        let dir = setup();
        SshKeyVault::init("my_pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("my_pass", dir.path());
        assert!(session.is_ok());
    }

    #[test]
    fn unlock_wrong_passphrase() {
        let dir = setup();
        SshKeyVault::init("correct", dir.path()).unwrap();
        let result = SshKeyVault::unlock("wrong", dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn generate_ed25519_key() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput {
                name: "Test Key".into(),
                algorithm: KeyAlgorithm::Ed25519,
                comment: Some("test@host".into()),
                export_policy: None,
            },
            dir.path(),
        )
        .unwrap();

        assert_eq!(item.name, "Test Key");
        assert_eq!(item.algorithm, KeyAlgorithm::Ed25519);
        assert!(item.fingerprint.starts_with("SHA256:"));
        assert!(item.public_key_openssh.starts_with("ssh-ed25519 "));
        assert!(item.public_key_openssh.contains("test@host"));
        assert_eq!(item.state, KeyState::Active);

        // Verify enc file exists
        assert!(dir.path().join("keys").join(format!("{}.enc", item.id)).exists());
    }

    #[test]
    fn list_keys_returns_all() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Key A".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Key B".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        let keys = SshKeyVault::list_keys(dir.path()).unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn get_key_returns_correct() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let created = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "My Key".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        let fetched = SshKeyVault::get_key(&created.id, dir.path()).unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "My Key");
    }

    #[test]
    fn delete_key_removes_from_list_and_file() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Temp".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        let enc_path = dir.path().join("keys").join(format!("{}.enc", item.id));
        assert!(enc_path.exists());

        SshKeyVault::delete_key(&session, &item.id, dir.path()).unwrap();

        assert!(!enc_path.exists());
        let keys = SshKeyVault::list_keys(dir.path()).unwrap();
        assert_eq!(keys.len(), 0);
    }

    #[test]
    fn archive_key_changes_state() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Arch".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        assert_eq!(item.state, KeyState::Active);

        SshKeyVault::archive_key(&item.id, dir.path()).unwrap();

        let fetched = SshKeyVault::get_key(&item.id, dir.path()).unwrap();
        assert_eq!(fetched.state, KeyState::Archived);
    }

    #[test]
    fn export_public_key() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Pub".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        let pub_key = SshKeyVault::export_public_key(&item.id, dir.path()).unwrap();
        assert_eq!(pub_key, item.public_key_openssh);
    }

    #[test]
    fn export_private_key_round_trip() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Priv".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        let pem = SshKeyVault::export_private_key(&session, &item.id, dir.path()).unwrap();
        // Verify it parses back
        let parsed = PrivateKey::from_openssh(&pem).unwrap();
        assert_eq!(
            parsed.fingerprint(HashAlg::Sha256).to_string(),
            item.fingerprint
        );
    }

    #[test]
    fn export_private_key_denied_by_policy() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput {
                name: "NoExport".into(),
                algorithm: KeyAlgorithm::Ed25519,
                comment: None,
                export_policy: Some(ExportPolicy { allow_private_export: false }),
            },
            dir.path(),
        ).unwrap();

        let result = SshKeyVault::export_private_key(&session, &item.id, dir.path());
        assert!(matches!(result, Err(VaultError::ExportDenied)));
    }

    #[test]
    fn change_passphrase_works() {
        let dir = setup();
        SshKeyVault::init("old_pass", dir.path()).unwrap();

        // Generate a key with old passphrase
        let session = SshKeyVault::unlock("old_pass", dir.path()).unwrap();
        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Change".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();
        drop(session);

        SshKeyVault::change_passphrase("old_pass", "new_pass", dir.path()).unwrap();

        // Old passphrase should fail
        assert!(SshKeyVault::unlock("old_pass", dir.path()).is_err());

        // New passphrase should work and decrypt existing keys
        let new_session = SshKeyVault::unlock("new_pass", dir.path()).unwrap();
        let pem = SshKeyVault::export_private_key(&new_session, &item.id, dir.path()).unwrap();
        assert!(pem.contains("OPENSSH PRIVATE KEY"));
    }

    #[test]
    fn update_key_metadata() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        let item = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Original".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        let updated = SshKeyVault::update_key(
            &item.id,
            UpdateKeyInput {
                name: Some("Renamed".into()),
                comment: Some("new comment".into()),
                export_policy: None,
            },
            dir.path(),
        ).unwrap();

        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.comment, "new comment");
        assert!(updated.updated_at > item.updated_at);
    }

    #[test]
    fn duplicate_name_rejected() {
        let dir = setup();
        SshKeyVault::init("pass", dir.path()).unwrap();
        let session = SshKeyVault::unlock("pass", dir.path()).unwrap();

        SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Same Name".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        ).unwrap();

        let result = SshKeyVault::generate_key(
            &session,
            GenerateKeyInput { name: "Same Name".into(), algorithm: KeyAlgorithm::Ed25519, comment: None, export_policy: None },
            dir.path(),
        );

        assert!(matches!(result, Err(VaultError::DuplicateKeyName(_))));
    }
}
