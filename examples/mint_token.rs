use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;

#[derive(Serialize)]
struct Claims {}

fn main() {
    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    let token = encode(
        &Header::new(Algorithm::HS256),
        &Claims {},
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .expect("failed to sign token");

    println!("{token}");
}
