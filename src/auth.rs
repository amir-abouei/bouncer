use hyper::HeaderMap;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashSet;

pub const HEADER_NAME: &str = "x-bouncer-token";

static JWT_SECRET: Lazy<Option<Vec<u8>>> = Lazy::new(|| {
    std::env::var("JWT_SECRET")
        .ok()
        .filter(|s| !s.is_empty())
        .map(String::into_bytes)
});

pub fn secret_configured() -> bool {
    JWT_SECRET.is_some()
}

/// Deliberately empty: no claim is required or inspected, only the
/// signature. Any well-formed JSON payload deserializes into this
/// (unknown fields are ignored by serde's default behavior), so tokens
/// carrying exp/nbf/aud/whatever still validate — those values are just
/// never looked at.
#[derive(Deserialize)]
struct Claims {}

pub fn extract_token(headers: &HeaderMap) -> Option<&str> {
    headers.get(HEADER_NAME)?.to_str().ok().map(str::trim)
}

pub fn verify(token: &str, secret: &[u8]) -> anyhow::Result<()> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.validate_aud = false;
    validation.required_spec_claims = HashSet::new();
    decode::<Claims>(token, &DecodingKey::from_secret(secret), &validation)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("token validation failed: {e}"))
}

pub fn authorize(headers: &HeaderMap) -> anyhow::Result<()> {
    let secret = JWT_SECRET
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("JWT_SECRET not configured"))?;
    let token = extract_token(headers)
        .ok_or_else(|| anyhow::anyhow!("missing {HEADER_NAME} header"))?;
    verify(token, secret)
}
