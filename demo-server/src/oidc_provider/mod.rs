use axum::{routing::get, Json, Router};
use josekit::jwk::{
    alg::{ec::EcCurve, ec::EcKeyPair, ed::EdKeyPair, rsa::RsaKeyPair},
    Jwk,
};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{net::SocketAddr, thread, time::Duration};

const ISSUER_URI: &str = "http://localhost:3001";

/// OpenId Connect discovery (simplified for test purposes)
#[derive(Serialize, Clone)]
struct OidcDiscovery {
    issuer: String,
    jwks_uri: String,
}

/// discovery url handler
async fn discovery() -> Json<Value> {
    let d = OidcDiscovery {
        issuer: ISSUER_URI.to_owned(),
        jwks_uri: format!("{ISSUER_URI}/jwks"),
    };
    Json(json!(d))
}

#[derive(Debug, Eq, PartialEq, Clone, Deserialize, Serialize)]
struct JwkSet {
    keys: Vec<Jwk>,
}

/// jwk set endpoint handler
async fn jwks() -> Json<Value> {
    let mut kset = JwkSet { keys: Vec::<Jwk>::new() };

    let keypair = RsaKeyPair::from_pem(include_bytes!("../../../config/jwtRS256.key")).unwrap();
    let mut pk = keypair.to_jwk_public_key();
    pk.set_key_id("key-rsa");
    pk.set_algorithm("RS256");
    pk.set_key_use("sig");
    kset.keys.push(pk);

    let keypair = RsaKeyPair::from_pem(include_bytes!("../../../config/private_rsa_key_pkcs8.pem")).unwrap();
    let mut pk = keypair.to_jwk_public_key();
    pk.set_key_id("rsa01");
    pk.set_algorithm("RS256");
    pk.set_key_use("sig");
    kset.keys.push(pk);

    let keypair = EcKeyPair::from_pem(include_bytes!("../../../config/ec256-private.pem"), Some(EcCurve::P256)).unwrap();
    let mut pk = keypair.to_jwk_public_key();
    pk.set_key_id("key-ec");
    pk.set_algorithm("ES256");
    pk.set_key_use("sig");
    kset.keys.push(pk);

    let keypair = EcKeyPair::from_pem(include_bytes!("../../../config/private_ecdsa_key.pem"), Some(EcCurve::P256)).unwrap();
    let mut pk = keypair.to_jwk_public_key();
    pk.set_key_id("ec01");
    pk.set_algorithm("ES256");
    pk.set_key_use("sig");
    kset.keys.push(pk);

    let keypair = EdKeyPair::from_pem(include_bytes!("../../../config/ed25519-private.pem")).unwrap();
    let mut pk = keypair.to_jwk_public_key();
    pk.set_key_id("key-ed");
    pk.set_algorithm("EdDSA");
    pk.set_key_use("sig");
    kset.keys.push(pk);

    let keypair = EdKeyPair::from_pem(include_bytes!("../../../config/private_ed25519_key.pem")).unwrap();
    let mut pk = keypair.to_jwk_public_key();
    pk.set_key_id("ed01");
    pk.set_algorithm("EdDSA");
    pk.set_key_use("sig");
    kset.keys.push(pk);

    Json(json!(kset))
}

/// build a minimal JWT header
fn build_header(alg: Algorithm, kid: &str) -> Header {
    Header {
        typ: Some("JWT".to_string()),
        alg,
        kid: Some(kid.to_owned()),
        cty: None,
        jku: None,
        jwk: None,
        x5u: None,
        x5c: None,
        x5t: None,
        x5t_s256: None,
    }
}

/// token claims
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    iss: &'static str,
    sub: &'static str,
    exp: usize,
    nbf: usize,
}

/// handler issuing test tokens (this is not a standard endpoint)
pub async fn tokens() -> Json<Value> {
    let claims = Claims {
        iss: ISSUER_URI,
        sub: "b@b.com",
        exp: 2000000000, // May 2033
        nbf: 1516239022, // Jan 2018
    };

    let rsa_key = EncodingKey::from_rsa_pem(include_bytes!("../../../config/jwtRS256.key")).unwrap();
    let ec_key = EncodingKey::from_ec_pem(include_bytes!("../../../config/ec256-private.pem")).unwrap();
    let ed_key = EncodingKey::from_ed_pem(include_bytes!("../../../config/ed25519-private.pem")).unwrap();

    let rsa_token = encode(&build_header(Algorithm::RS256, "key-rsa"), &claims, &rsa_key).unwrap();
    let ec_token = encode(&build_header(Algorithm::ES256, "key-ec"), &claims, &ec_key).unwrap();
    let ed_token = encode(&build_header(Algorithm::EdDSA, "key-ed"), &claims, &ed_key).unwrap();

    Json(json!({
        "rsa": rsa_token,
        "ec": ec_token,
        "ed": ed_token
    }))
}

/// exposes some oidc "like" endpoints for test purposes
pub fn run_server() -> &'static str {
    let app = Router::new()
        .route("/.well-known/openid-configuration", get(discovery))
        .route("/jwks", get(jwks))
        .route("/tokens", get(tokens));

    tokio::spawn(async move {
        let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
        tracing::info!("oidc provider starting on: {}", addr);
        axum::Server::bind(&addr).serve(app.into_make_service()).await.unwrap();
    });

    thread::sleep(Duration::from_millis(200)); // waiting oidc to start

    ISSUER_URI
}
