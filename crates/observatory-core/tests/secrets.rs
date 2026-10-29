//! `core::secrets` (architecture §12): `GITHUB_TOKEN`, then the keychain; a value never prints.
//! No test here reads the real keychain: resolution takes the keychain as a closure and the
//! service lookup runs against keyring's mock store.

use std::cell::Cell;

use observatory_core::secrets::{
    Secret, SecretError, SecretName, TokenSource, lookup_service, resolve_github_token,
};

/// A PAT-shaped value built at run time: a literal would trip secret scanners on push.
fn fake_token() -> String {
    ["github", "pat", &"x".repeat(40)].join("_")
}

fn refusal(message: &str) -> SecretError {
    SecretError::Keychain {
        service: "droid-observatory/github-token".into(),
        message: message.into(),
    }
}

#[test]
fn a_secret_prints_as_redacted_in_debug_and_display() {
    let value = fake_token();
    let secret = Secret::new(value.clone());
    assert_eq!(format!("{secret}"), "<redacted>");
    assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
    assert!(!format!("{:?}", Some(&secret)).contains("github_pat_"));
    assert_eq!(secret.expose(), value);
}

#[test]
fn secret_names_map_to_the_keychain_service() {
    assert_eq!(
        SecretName::GithubToken.service(),
        "droid-observatory/github-token"
    );
    assert_eq!(
        SecretName::OtlpHeaders.service(),
        "droid-observatory/otlp-headers"
    );
}

#[test]
fn a_service_without_an_item_is_none_not_an_error() {
    keyring_core::set_default_store(keyring_core::mock::Store::new().unwrap());
    let service = format!("droid-observatory/test-absent-{}", std::process::id());
    assert!(lookup_service(&service).unwrap().is_none());
    let store = keyring_core::get_default_store().unwrap();
    assert!(
        store.vendor().starts_with("Mock store"),
        "the injected store"
    );
}

#[test]
fn the_environment_wins_and_the_keychain_is_not_consulted() {
    let calls = Cell::new(0);
    let fake_keychain = || {
        calls.set(calls.get() + 1);
        Ok(Some(Secret::new("from-keychain".into())))
    };
    let (token, source) = resolve_github_token(Some("from-env".into()), fake_keychain).unwrap();
    assert_eq!(token.unwrap().expose(), "from-env");
    assert_eq!(source, TokenSource::Env);
    assert_eq!(
        calls.get(),
        0,
        "a set GITHUB_TOKEN leaves the keychain alone"
    );

    let (token, source) = resolve_github_token(Some(String::new()), fake_keychain).unwrap();
    assert_eq!(token.unwrap().expose(), "from-keychain");
    assert_eq!(source, TokenSource::Keychain);
    let (_, source) = resolve_github_token(None, fake_keychain).unwrap();
    assert_eq!(source, TokenSource::Keychain);
    assert_eq!(calls.get(), 2, "an empty or unset value reads it once each");
}

#[test]
fn the_environment_is_the_fallback_and_an_empty_value_is_no_token() {
    let (token, source) = resolve_github_token(Some("from-env".into()), || Ok(None)).unwrap();
    assert_eq!(token.unwrap().expose(), "from-env");
    assert_eq!(source, TokenSource::Env);
    let (token, source) = resolve_github_token(Some(String::new()), || Ok(None)).unwrap();
    assert!(token.is_none());
    assert_eq!(source, TokenSource::None);
    let (token, _) = resolve_github_token(None, || Ok(None)).unwrap();
    assert!(token.is_none());
}

#[test]
fn a_refusing_keychain_falls_back_to_the_environment() {
    let (token, source) =
        resolve_github_token(Some("from-env".into()), || Err(refusal("canceled"))).unwrap();
    assert_eq!(token.unwrap().expose(), "from-env");
    assert_eq!(source, TokenSource::Env);
    assert!(
        resolve_github_token(None, || Err(refusal("locked"))).is_err(),
        "no environment value: the refusal is the answer"
    );
    assert!(
        resolve_github_token(Some(String::new()), || Err(refusal("locked"))).is_err(),
        "an empty environment value is no value"
    );
}
