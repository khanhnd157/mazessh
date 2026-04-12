use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::error::MazeSshError;

const SERVICE_NAME: &str = "maze-ssh";
const PIN_HASH_KEY: &str = "pin-hash";

pub fn set_pin(pin: &str) -> Result<(), MazeSshError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(pin.as_bytes(), &salt)
        .map_err(|e| MazeSshError::SecurityError(format!("Failed to hash PIN: {}", e)))?;

    let entry = keyring::Entry::new(SERVICE_NAME, PIN_HASH_KEY)
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;
    entry
        .set_password(&hash.to_string())
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;

    Ok(())
}

pub fn verify_pin(pin: &str) -> Result<bool, MazeSshError> {
    let entry = keyring::Entry::new(SERVICE_NAME, PIN_HASH_KEY)
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;

    let stored_hash = match entry.get_password() {
        Ok(h) => h,
        Err(keyring::Error::NoEntry) => return Ok(false),
        Err(e) => return Err(MazeSshError::KeyringError(e.to_string())),
    };

    let parsed_hash = PasswordHash::new(&stored_hash)
        .map_err(|e| MazeSshError::SecurityError(format!("Invalid stored hash: {}", e)))?;

    Ok(Argon2::default()
        .verify_password(pin.as_bytes(), &parsed_hash)
        .is_ok())
}

pub fn is_pin_configured() -> bool {
    let entry = match keyring::Entry::new(SERVICE_NAME, PIN_HASH_KEY) {
        Ok(e) => e,
        Err(_) => return false,
    };
    matches!(entry.get_password(), Ok(_))
}

pub fn remove_pin() -> Result<(), MazeSshError> {
    let entry = keyring::Entry::new(SERVICE_NAME, PIN_HASH_KEY)
        .map_err(|e| MazeSshError::KeyringError(e.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(MazeSshError::KeyringError(e.to_string())),
    }
}
