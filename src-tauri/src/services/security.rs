use zeroize::Zeroizing;

use crate::error::MazeSshError;

const SERVICE_NAME: &str = "maze-ssh";

pub fn store_passphrase(profile_id: &str, passphrase: &str) -> Result<(), MazeSshError> {
    let entry = keyring::Entry::new(SERVICE_NAME, profile_id)
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;
    entry
        .set_password(passphrase)
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;
    Ok(())
}

/// Retrieve a stored SSH key passphrase from the system credential store.
/// Returns `Zeroizing<String>` so the passphrase bytes are cleared when the caller drops the value.
pub fn get_passphrase(profile_id: &str) -> Result<Option<Zeroizing<String>>, MazeSshError> {
    let entry = keyring::Entry::new(SERVICE_NAME, profile_id)
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;
    match entry.get_password() {
        Ok(pass) => Ok(Some(Zeroizing::new(pass))),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(MazeSshError::KeyringError(e.to_string())),
    }
}

pub fn delete_passphrase(profile_id: &str) -> Result<(), MazeSshError> {
    let entry = keyring::Entry::new(SERVICE_NAME, profile_id)
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(MazeSshError::KeyringError(e.to_string())),
    }
}
