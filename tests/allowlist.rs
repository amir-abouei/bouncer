use bouncer::allowlist::{check_fail_closed, Target};
use std::collections::HashMap;

#[test]
fn parses_target_with_auth() {
    let target: Target = toml::from_str(
        r#"host = "example.com"
auth = true"#,
    )
    .unwrap();
    assert_eq!(target.host, "example.com");
    assert!(target.auth);
}

#[test]
fn target_auth_defaults_to_false() {
    let target: Target = toml::from_str(r#"host = "example.com""#).unwrap();
    assert!(!target.auth);
}

#[test]
fn fail_closed_ok_when_no_target_requires_auth() {
    let mut targets = HashMap::new();
    targets.insert(
        "foo".to_string(),
        Target {
            host: "example.com".to_string(),
            auth: false,
        },
    );
    assert!(check_fail_closed(&targets, false).is_ok());
}

#[test]
fn fail_closed_ok_when_secret_configured() {
    let mut targets = HashMap::new();
    targets.insert(
        "foo".to_string(),
        Target {
            host: "example.com".to_string(),
            auth: true,
        },
    );
    assert!(check_fail_closed(&targets, true).is_ok());
}

#[test]
fn fail_closed_errors_naming_offending_alias() {
    let mut targets = HashMap::new();
    targets.insert(
        "foo".to_string(),
        Target {
            host: "example.com".to_string(),
            auth: true,
        },
    );
    let err = check_fail_closed(&targets, false).unwrap_err();
    assert!(err.to_string().contains("foo"));
}
