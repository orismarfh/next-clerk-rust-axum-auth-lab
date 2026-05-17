use std::{collections::HashMap, sync::Arc, time::Duration};

use async_trait::async_trait;
use axum::{
    body::Body,
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, HeaderMap, Request},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use tokio::time::Instant;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub issuer: String,
    pub jwks_url: String,
    pub authorized_parties: Vec<String>,
}

impl AuthConfig {
    pub fn from_env() -> Result<Self, AppError> {
        let issuer = std::env::var("CLERK_JWT_ISSUER")
            .map_err(|_| AppError::Internal("CLERK_JWT_ISSUER not set".into()))?;
        let issuer = issuer.trim_end_matches('/').to_string();
        let jwks_url = format!("{}/.well-known/jwks.json", issuer);
        let authorized_parties = std::env::var("CLERK_AUTHORIZED_PARTIES")
            .map_err(|_| AppError::Internal("CLERK_AUTHORIZED_PARTIES not set".into()))?
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>();
        if authorized_parties.is_empty() {
            return Err(AppError::Internal(
                "CLERK_AUTHORIZED_PARTIES must list at least one origin".into(),
            ));
        }
        Ok(Self {
            issuer,
            jwks_url,
            authorized_parties,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub iss: String,
    pub exp: usize,
    #[serde(default)]
    pub nbf: Option<usize>,
    #[serde(default)]
    pub iat: Option<usize>,
    #[serde(default)]
    pub azp: Option<String>,
    #[serde(default)]
    pub sid: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct Jwk {
    kid: String,
    #[serde(default)]
    kty: String,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct JwksDoc {
    keys: Vec<Jwk>,
}

const JWKS_TTL: Duration = Duration::from_secs(5 * 60);

pub struct JwksCache {
    keys: RwLock<HashMap<String, DecodingKey>>,
    last_refresh: RwLock<Option<Instant>>,
    http: reqwest::Client,
    jwks_url: String,
}

impl JwksCache {
    pub fn new(jwks_url: String) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("failed to build reqwest client");
        Self {
            keys: RwLock::new(HashMap::new()),
            last_refresh: RwLock::new(None),
            http,
            jwks_url,
        }
    }

    pub async fn get_or_refresh(&self, kid: &str) -> Result<DecodingKey, AppError> {
        let stale = {
            let last = self.last_refresh.read().await;
            match *last {
                Some(t) => t.elapsed() >= JWKS_TTL,
                None => true,
            }
        };

        if !stale {
            if let Some(key) = self.keys.read().await.get(kid).cloned() {
                return Ok(key);
            }
        }

        self.refresh().await?;

        self.keys
            .read()
            .await
            .get(kid)
            .cloned()
            .ok_or_else(|| AppError::Unauthorized(format!("unknown key id: {kid}")))
    }

    async fn refresh(&self) -> Result<(), AppError> {
        tracing::debug!(url = %self.jwks_url, "refreshing JWKS");
        let resp = self
            .http
            .get(&self.jwks_url)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("JWKS fetch failed: {e}")))?;
        if !resp.status().is_success() {
            return Err(AppError::Internal(format!(
                "JWKS endpoint returned {}",
                resp.status()
            )));
        }
        let doc: JwksDoc = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("JWKS parse failed: {e}")))?;

        let mut new_keys = HashMap::with_capacity(doc.keys.len());
        for jwk in doc.keys {
            if !jwk.kty.is_empty() && jwk.kty != "RSA" {
                continue;
            }
            match DecodingKey::from_rsa_components(&jwk.n, &jwk.e) {
                Ok(key) => {
                    new_keys.insert(jwk.kid, key);
                }
                Err(e) => {
                    tracing::warn!(error = %e, "skipping JWK with invalid RSA components");
                }
            }
        }

        let mut keys = self.keys.write().await;
        *keys = new_keys;
        let mut last = self.last_refresh.write().await;
        *last = Some(Instant::now());
        Ok(())
    }
}

pub async fn verify_token(
    token: &str,
    cache: &JwksCache,
    cfg: &AuthConfig,
) -> Result<Claims, AppError> {
    let header =
        decode_header(token).map_err(|e| AppError::Unauthorized(format!("bad JWT header: {e}")))?;
    let kid = header
        .kid
        .ok_or_else(|| AppError::Unauthorized("JWT header missing kid".into()))?;

    let key = cache.get_or_refresh(&kid).await?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_issuer(&[cfg.issuer.as_str()]);
    validation.set_required_spec_claims(&["exp", "iss"]);

    let data = decode::<Claims>(token, &key, &validation)
        .map_err(|e| AppError::Unauthorized(format!("JWT verification failed: {e}")))?;

    let claims = data.claims;

    match &claims.azp {
        Some(azp) if cfg.authorized_parties.iter().any(|p| p == azp) => {}
        Some(azp) => {
            return Err(AppError::Forbidden(format!(
                "azp '{azp}' is not in the authorized parties allowlist"
            )));
        }
        None => {
            return Err(AppError::Forbidden(
                "JWT missing azp claim; cannot verify caller origin".into(),
            ));
        }
    }

    Ok(claims)
}

#[derive(Clone)]
pub struct AppState {
    pub jwks: Arc<JwksCache>,
    pub cfg: Arc<AuthConfig>,
}

pub fn extract_bearer_token(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer ").or_else(|| h.strip_prefix("bearer ")))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .ok_or_else(|| AppError::Unauthorized("missing Bearer token".into()))
}

pub async fn require_auth(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = extract_bearer_token(req.headers())?;
    let claims = verify_token(&token, &state.jwks, &state.cfg).await?;
    req.extensions_mut().insert(claims);
    Ok(next.run(req).await)
}

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or_else(|| AppError::Unauthorized("claims not present in request".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_with(value: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert(AUTHORIZATION, HeaderValue::from_str(value).unwrap());
        h
    }

    #[test]
    fn extracts_with_capital_bearer_prefix() {
        let h = headers_with("Bearer abc.def.ghi");
        assert_eq!(extract_bearer_token(&h).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn extracts_with_lowercase_bearer_prefix() {
        let h = headers_with("bearer abc.def.ghi");
        assert_eq!(extract_bearer_token(&h).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn trims_surrounding_whitespace() {
        let h = headers_with("Bearer    abc.def.ghi\t");
        assert_eq!(extract_bearer_token(&h).unwrap(), "abc.def.ghi");
    }

    #[test]
    fn rejects_missing_header() {
        let h = HeaderMap::new();
        let err = extract_bearer_token(&h).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn rejects_wrong_scheme() {
        let h = headers_with("Basic dXNlcjpwYXNz");
        let err = extract_bearer_token(&h).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn rejects_empty_token_after_prefix() {
        let h = headers_with("Bearer    ");
        let err = extract_bearer_token(&h).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }
}
