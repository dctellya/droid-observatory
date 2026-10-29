//! Secrets from the OS keychain (architecture §12). A value never prints: `Secret` formats as
//! `<redacted>`, and the only way to read it is `expose`, which the HTTP client calls once per
//! request to build a header.

use std::fmt;
use std::sync::Once;

/// The keychain items the app reads: generic passwords with service `droid-observatory/<name>`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecretName {
    GithubToken,
    OtlpHeaders,
}

impl SecretName {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::GithubToken => "github-token",
            Self::OtlpHeaders => "otlp-headers",
        }
    }

    pub fn service(self) -> String {
        format!("droid-observatory/{}", self.as_str())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    #[error("the keychain refused the lookup of {service}: {message}")]
    Keychain { service: String, message: String },
}

/// A secret value. It has no `Clone`, `Display` of the inner text or `Serialize`: a stray
/// `{:?}` or a serialised struct cannot leak it.
pub struct Secret(String);

impl Secret {
    pub fn new(value: String) -> Self {
        Self(value)
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(<redacted>)")
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<redacted>")
    }
}

/// The keychain item `name`, if there is one.
pub fn lookup(name: SecretName) -> Result<Option<Secret>, SecretError> {
    lookup_service(&name.service())
}

/// The keychain item of an exact service name. Absent items and platforms without a keychain
/// are `None`, not errors: a missing secret is a normal state (telemetry stays off, the gate
/// lane reports it needs a token).
pub fn lookup_service(service: &str) -> Result<Option<Secret>, SecretError> {
    install_store();
    let account = std::env::var("USER").unwrap_or_default();
    let refused = |error: keyring_core::Error| SecretError::Keychain {
        service: service.to_owned(),
        message: error.to_string(),
    };
    let entry = match keyring_core::Entry::new(service, &account) {
        Ok(entry) => entry,
        Err(keyring_core::Error::NoDefaultStore) => return Ok(None),
        Err(error) => return Err(refused(error)),
    };
    match entry.get_password() {
        Ok(value) => Ok(Some(Secret(value))),
        Err(keyring_core::Error::NoEntry) => Ok(None),
        Err(error) => Err(refused(error)),
    }
}

/// The GitHub token: the keychain, then `GITHUB_TOKEN` (the only environment fallback). A
/// keychain that refuses (locked, access denied) gives way to the environment value; with none
/// the refusal is returned.
pub fn github_token() -> Result<Option<Secret>, SecretError> {
    resolve_github_token_or_fallback(
        lookup(SecretName::GithubToken),
        std::env::var("GITHUB_TOKEN").ok(),
    )
}

pub fn resolve_github_token_or_fallback(
    keychain: Result<Option<Secret>, SecretError>,
    env: Option<String>,
) -> Result<Option<Secret>, SecretError> {
    match keychain {
        Ok(found) => Ok(resolve_github_token(found, env)),
        Err(refusal) => match resolve_github_token(None, env) {
            Some(token) => {
                tracing::warn!(%refusal, "keychain refused, using GITHUB_TOKEN");
                Ok(Some(token))
            }
            None => Err(refusal),
        },
    }
}

/// The keychain value when there is one, else a non-empty environment value.
pub fn resolve_github_token(keychain: Option<Secret>, env: Option<String>) -> Option<Secret> {
    keychain.or_else(|| env.filter(|value| !value.is_empty()).map(Secret))
}

/// The OTLP headers: the keychain only (D-010).
pub fn otlp_headers() -> Result<Option<Secret>, SecretError> {
    lookup(SecretName::OtlpHeaders)
}

#[cfg(target_os = "macos")]
fn install_store() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        if keyring_core::get_default_store().is_none()
            && let Ok(store) = apple_native_keyring_store::keychain::Store::new()
        {
            keyring_core::set_default_store(store);
        }
    });
}

#[cfg(not(target_os = "macos"))]
fn install_store() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {});
}
