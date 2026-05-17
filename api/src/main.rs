use std::{net::SocketAddr, sync::Arc};

use axum::{
    http::{header, HeaderValue, Method},
    middleware,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{prelude::*, EnvFilter};

mod auth;
mod error;

use auth::{require_auth, AppState, AuthConfig, Claims, JwksCache};
use error::AppError;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,api=debug")))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cfg = AuthConfig::from_env().map_err(|e| {
        eprintln!("config error: {e}");
        e
    })?;
    let jwks = Arc::new(JwksCache::new(cfg.jwks_url.clone()));
    let state = AppState {
        jwks: jwks.clone(),
        cfg: Arc::new(cfg.clone()),
    };

    let cors_origins: Vec<HeaderValue> = cfg
        .authorized_parties
        .iter()
        .filter_map(|origin| HeaderValue::from_str(origin).ok())
        .collect();
    let cors = CorsLayer::new()
        .allow_origin(cors_origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    let public = Router::new().route("/health", get(health));
    let protected = Router::new()
        .route("/me", get(me))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth));

    let app = public
        .merge(protected)
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let port: u16 = std::env::var("API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}

async fn me(claims: Claims) -> Result<Json<Claims>, AppError> {
    Ok(Json(claims))
}
