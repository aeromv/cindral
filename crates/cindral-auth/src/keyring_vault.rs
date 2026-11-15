// SPDX-License-Identifier: MIT OR Apache-2.0
//! [`SecretVault`] in the system keychain (Secret Service on Linux, Keychain
//! on macOS, Credential Manager on Windows), through the `keyring` crate.

use keyring::Entry;

use crate::{error::AuthError, secret::Secret, vault::SecretVault};

/// Service name under which all entries are filed.
const SERVICE: &str = "cindral";
/// Service name of the entries saved before the client was named Cindral.
const LEGACY_SERVICE: &str = "umicraft";

/// Keeps secrets in the operating system's keychain.
#[derive(Debug, Clone, Copy, Default)]
pub struct KeyringVault;

impl KeyringVault {
    /// Checks that a keychain is available, so the caller can fall back to a
    /// [`MemoryVault`](crate::vault::MemoryVault) when it is not.
    ///
    /// # Errors
    /// [`AuthError::Storage`] when no keychain backend can start.
    pub fn probe() -> Result<Self, AuthError> {
        entry("probe")?;
        Ok(Self)
    }
}

fn entry(key: &str) -> Result<Entry, AuthError> {
    Entry::new(SERVICE, key).map_err(storage_error)
}

/// Reads `key` under the old service name and, when found, files it under
/// the new one, so a signed-in account survives the rename. The old entry is
/// removed only once the copy is saved.
fn migrate_legacy(key: &str) -> Result<Option<Secret>, AuthError> {
    let legacy = Entry::new(LEGACY_SERVICE, key).map_err(storage_error)?;
    let value = match legacy.get_password() {
        Ok(value) => value,
        Err(keyring::Error::NoEntry) => return Ok(None),
        Err(error) => return Err(storage_error(error)),
    };
    if entry(key)?.set_password(&value).is_ok() {
        let _ = legacy.delete_credential();
    }
    Ok(Some(Secret::new(value)))
}

fn storage_error(error: keyring::Error) -> AuthError {
    AuthError::Storage(format!("system keychain: {error}"))
}

impl SecretVault for KeyringVault {
    fn get(&self, key: &str) -> Result<Option<Secret>, AuthError> {
        match entry(key)?.get_password() {
            Ok(value) => Ok(Some(Secret::new(value))),
            Err(keyring::Error::NoEntry) => migrate_legacy(key),
            Err(error) => Err(storage_error(error)),
        }
    }

    fn set(&self, key: &str, value: &Secret) -> Result<(), AuthError> {
        entry(key)?
            .set_password(value.expose())
            .map_err(storage_error)
    }

    fn remove(&self, key: &str) -> Result<(), AuthError> {
        match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(storage_error(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_errors_keep_their_kind_without_values() {
        let err = storage_error(keyring::Error::NoEntry);
        assert!(matches!(err, AuthError::Storage(_)));
    }
}
