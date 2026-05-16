use axum::{
    extract::State,
    http::{header::AUTHORIZATION, HeaderMap, StatusCode},
    routing::get,
    Json, Router,
};
use serde::Serialize;
use std::{env, net::SocketAddr};

#[derive(Clone)]
struct AppState {
    clerk_jwks_url: String,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Serialize)]
struct ProtectedResponse {
    message: String,
    token_preview: String,
}

pub fn app() -> Router {
    let clerk_jwks_url = env::var("CLERK_JWKS_URL")
        .unwrap_or_else(|_| "https://YOUR_CLERK_DOMAIN/.well-known/jwks.json".to_string());

    Router::new()
        .route("/api/public", get(public_route))
        .route("/api/protected", get(protected_route))
        .with_state(AppState { clerk_jwks_url })
}

async fn public_route() -> Json<MessageResponse> {
    Json(MessageResponse {
        message: "This is a public Axum route. No token required.".to_string(),
    })
}

async fn protected_route(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<ProtectedResponse>, StatusCode> {
    let token = extract_bearer_token(&headers)?;

    // IMPORTANT: This lab intentionally does not perform real JWT validation.
    // In production, validate `token` against Clerk's JWKS (`state.clerk_jwks_url`) and enforce
    // issuer, audience, expiration, and signature checks before authorizing the request.
    let token_preview: String = token.chars().take(12).collect();

    Ok(Json(ProtectedResponse {
        message: format!(
            "Bearer token detected. Real Clerk JWT validation should use {}",
            state.clerk_jwks_url
        ),
        token_preview,
    }))
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<String, StatusCode> {
    let auth_header = headers
        .get(AUTHORIZATION)
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .trim();

    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(token.to_string())
}

#[tokio::main]
async fn main() {
    let app = app();

    let port = env::var("PORT")
        .ok()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(4000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind TCP listener");

    println!("Backend running on http://{}", addr);

    axum::serve(listener, app)
        .await
        .expect("backend server failed");
}

#[cfg(test)]
mod tests {
    use super::app;
    use axum::{
        body::{to_bytes, Body},
        http::{header::AUTHORIZATION, Request, StatusCode},
    };
    use serde_json::Value;
    use tower::ServiceExt;

    #[tokio::test]
    async fn public_route_is_accessible_without_auth() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/public")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["message"], "This is a public Axum route. No token required.");
    }

    #[tokio::test]
    async fn protected_route_rejects_missing_token() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_route_accepts_bearer_header() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/api/protected")
                    .header(AUTHORIZATION, "Bearer fake.clerk.jwt")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let payload: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["token_preview"], "fake.clerk.j");
    }
}
