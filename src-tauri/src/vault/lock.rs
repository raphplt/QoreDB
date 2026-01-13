//! Vault Lock
//!
//! Master password protection for the vault at startup.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use keyring::Entry;

use crate::engine::error::{EngineError, EngineResult};

const SERVICE_NAME: &str = "qoredb";
const MASTER_PASSWORD_KEY: &str = "__master_password_hash__";

/// Manages vault locking with master password
pub struct VaultLock {
    is_unlocked: bool,
}

impl VaultLock {
    pub fn new() -> Self {
        Self { is_unlocked: false }
    }

    /// Checks if a master password has been set
    pub fn has_master_password() -> EngineResult<bool> {
        let entry = Entry::new(SERVICE_NAME, MASTER_PASSWORD_KEY)
            .map_err(|e| EngineError::internal(format!("Keyring error: {}", e)))?;

        match entry.get_password() {
            Ok(_) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(EngineError::internal(format!("Keyring error: {}", e))),
        }
    }

    /// Sets up a new master password
    pub fn setup_master_password(&mut self, password: &str) -> EngineResult<()> {
        // Hash the password with Argon2
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        
        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| EngineError::internal(format!("Hashing error: {}", e)))?
            .to_string();

        // Store the hash in keyring
        let entry = Entry::new(SERVICE_NAME, MASTER_PASSWORD_KEY)
            .map_err(|e| EngineError::internal(format!("Keyring error: {}", e)))?;

        entry
            .set_password(&hash)
            .map_err(|e| EngineError::internal(format!("Failed to store master password: {}", e)))?;

        self.is_unlocked = true;
        Ok(())
    }

    /// Attempts to unlock the vault with the given password
    pub fn unlock(&mut self, password: &str) -> EngineResult<bool> {
        let entry = Entry::new(SERVICE_NAME, MASTER_PASSWORD_KEY)
            .map_err(|e| EngineError::internal(format!("Keyring error: {}", e)))?;

        let stored_hash = entry
            .get_password()
            .map_err(|e| EngineError::internal(format!("No master password set: {}", e)))?;

        let parsed_hash = PasswordHash::new(&stored_hash)
            .map_err(|e| EngineError::internal(format!("Invalid stored hash: {}", e)))?;

        let argon2 = Argon2::default();
        
        if argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok() {
            self.is_unlocked = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Locks the vault
    pub fn lock(&mut self) {
        self.is_unlocked = false;
    }

    /// Checks if the vault is currently unlocked
    pub fn is_locked(&self) -> bool {
        !self.is_unlocked
    }

    /// Checks if the vault is currently unlocked
    pub fn is_unlocked(&self) -> bool {
        self.is_unlocked
    }

    /// Removes the master password (requires current password)
    pub fn remove_master_password(&mut self, password: &str) -> EngineResult<()> {
        // Verify current password first
        if !self.unlock(password)? {
            return Err(EngineError::auth_failed("Invalid password"));
        }

        let entry = Entry::new(SERVICE_NAME, MASTER_PASSWORD_KEY)
            .map_err(|e| EngineError::internal(format!("Keyring error: {}", e)))?;

        entry
            .delete_credential()
            .map_err(|e| EngineError::internal(format!("Failed to delete: {}", e)))?;

        self.is_unlocked = true; // No password = always unlocked
        Ok(())
    }

    /// Auto-unlocks if no master password is set
    pub fn auto_unlock_if_no_password(&mut self) -> EngineResult<()> {
        if !Self::has_master_password()? {
            self.is_unlocked = true;
        }
        Ok(())
    }
}

impl Default for VaultLock {
    fn default() -> Self {
        Self::new()
    }
}
