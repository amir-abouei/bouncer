use bouncer::auth::{extract_token, verify, HEADER_NAME};
use hyper::header::HeaderValue;
use hyper::HeaderMap;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

fn sign(
    exp: Option<u64>,
    nbf: Option<u64>,
    aud: Option<&str>,
    secret: &[u8],
) -> String {
    #[derive(serde::Serialize)]
    struct FullClaims<'a> {
        #[serde(skip_serializing_if = "Option::is_none")]
        exp: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        nbf: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        aud: Option<&'a str>,
    }
    encode(
        &Header::new(Algorithm::HS256),
        &FullClaims { exp, nbf, aud },
        &EncodingKey::from_secret(secret),
    )
    .unwrap()
}

#[test]
fn extract_token_missing_header() {
    let headers = HeaderMap::new();
    assert!(extract_token(&headers).is_none());
}

#[test]
fn extract_token_well_formed() {
    let mut headers = HeaderMap::new();
    headers.insert(HEADER_NAME, HeaderValue::from_static("abc123"));
    assert_eq!(extract_token(&headers), Some("abc123"));
}

#[test]
fn verify_accepts_token_with_no_claims() {
    let secret = b"test-secret";
    let token = sign(None, None, None, secret);
    assert!(verify(&token, secret).is_ok());
}

#[test]
fn verify_ignores_expired_exp_claim() {
    let secret = b"test-secret";
    let token = sign(Some(now() - 3600), None, None, secret);
    assert!(verify(&token, secret).is_ok());
}

#[test]
fn verify_ignores_future_nbf_claim() {
    let secret = b"test-secret";
    let token = sign(None, Some(now() + 3600), None, secret);
    assert!(verify(&token, secret).is_ok());
}

#[test]
fn verify_ignores_unconfigured_audience() {
    let secret = b"test-secret";
    let token = sign(None, None, Some("some-audience"), secret);
    assert!(verify(&token, secret).is_ok());
}

#[test]
fn verify_rejects_wrong_secret() {
    let token = sign(None, None, None, b"right-secret");
    assert!(verify(&token, b"wrong-secret").is_err());
}

#[test]
fn verify_rejects_malformed_token() {
    assert!(verify("not-a-jwt", b"test-secret").is_err());
}
