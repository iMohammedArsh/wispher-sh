//! API keys in the OS keychain (macOS Keychain / Windows Credential Manager).

use hams_core::error::{AppError, AppResult};
use hams_core::settings::SecretStore;

const KEYCHAIN_SERVICE: &str = "com.wusla.hams";
/// Earlier v2 builds used this service name; keys found there are moved over.
const LEGACY_KEYCHAIN_SERVICE: &str = "com.mohammedarsh.hams";

pub struct KeychainSecrets;

impl SecretStore for KeychainSecrets {
    fn get(&self, account: &str) -> Option<String> {
        get_secret(account)
    }

    fn set(&self, account: &str, value: &str) -> AppResult<()> {
        set_secret(account, value)
    }

    fn has(&self, account: &str) -> bool {
        has_secret(account)
    }
}

fn set_secret(account: &str, value: &str) -> AppResult<()> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, account).map_err(|e| AppError::Settings(e.to_string()))?;
    if value.trim().is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(AppError::Settings(e.to_string())),
        }
    } else {
        entry.set_password(value.trim()).map_err(|e| AppError::Settings(e.to_string()))
    }
}

fn read_secret(service: &str, account: &str) -> Option<String> {
    keyring::Entry::new(service, account)
        .ok()?
        .get_password()
        .ok()
        .filter(|s| !s.is_empty())
}

/// Whether a key is stored, without reading it. On macOS this asks the keychain
/// for the item's attributes only: reading the secret itself makes macOS ask for
/// the login password whenever the app's signature differs from the one that
/// saved it, and this is called every time the settings load.
fn has_secret(account: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        use security_framework::item::{ItemClass, ItemSearchOptions};
        [KEYCHAIN_SERVICE, LEGACY_KEYCHAIN_SERVICE].iter().any(|service| {
            ItemSearchOptions::new()
                .class(ItemClass::generic_password())
                .service(service)
                .account(account)
                .load_attributes(true)
                .limit(1)
                .search()
                .is_ok_and(|found| !found.is_empty())
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        get_secret(account).is_some()
    }
}

fn get_secret(account: &str) -> Option<String> {
    if let Some(secret) = read_secret(KEYCHAIN_SERVICE, account) {
        return Some(secret);
    }
    let legacy = read_secret(LEGACY_KEYCHAIN_SERVICE, account)?;
    if set_secret(account, &legacy).is_ok() {
        if let Ok(entry) = keyring::Entry::new(LEGACY_KEYCHAIN_SERVICE, account) {
            let _ = entry.delete_credential();
        }
        tracing::info!("Moved the {account} API key to the {KEYCHAIN_SERVICE} keychain entry");
    }
    Some(legacy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn has_secret_is_false_for_a_missing_key() {
        assert!(!has_secret("hams-test-account-that-does-not-exist"));
    }
}
