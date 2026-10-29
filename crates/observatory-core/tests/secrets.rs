//! `core::secrets` (architecture §12): the keychain, then `GITHUB_TOKEN`; a value never prints.

use observatory_core::secrets::{Secret, SecretName, lookup_service, resolve_github_token};

#[test]
fn a_secret_prints_as_redacted_in_debug_and_display() {
    let secret = Secret::new("github_pat_abcdefghijklmnopqrstuvwxyz0123456789".into());
    assert_eq!(format!("{secret}"), "<redacted>");
    assert_eq!(format!("{secret:?}"), "Secret(<redacted>)");
    assert!(!format!("{:?}", Some(&secret)).contains("github_pat_"));
    assert_eq!(
        secret.expose(),
        "github_pat_abcdefghijklmnopqrstuvwxyz0123456789"
    );
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
    let service = format!("droid-observatory/test-absent-{}", std::process::id());
    assert!(lookup_service(&service).unwrap().is_none());
}

#[test]
fn the_keychain_wins_over_the_environment() {
    let keychain = Some(Secret::new("from-keychain".into()));
    let token = resolve_github_token(keychain, Some("from-env".into())).unwrap();
    assert_eq!(token.expose(), "from-keychain");
}

#[test]
fn the_environment_is_the_fallback_and_an_empty_value_is_no_token() {
    let token = resolve_github_token(None, Some("from-env".into())).unwrap();
    assert_eq!(token.expose(), "from-env");
    assert!(resolve_github_token(None, Some(String::new())).is_none());
    assert!(resolve_github_token(None, None).is_none());
}
