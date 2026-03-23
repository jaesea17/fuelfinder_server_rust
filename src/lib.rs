#![forbid(clippy::unwrap_used)]

use axum::{
    Router,
    http::StatusCode,
    routing::get,
};
use http::{HeaderName, header::{AUTHORIZATION, CONTENT_TYPE}};
use std::collections::HashSet;
use tower_http::cors::{Any, CorsLayer};

pub mod app_state;
pub mod authentication;
pub mod domain;

use crate::{
    app_state::AppState,
    authentication::{
        admin::routes::admin_routes,
        station::authenticate::routes::auth_routes,
    },
    domain::{
        commodities::routes::commodities_route,
        discounts::routes::discounts_route,
        stations::routes::stations_route,
    },
};

pub fn listen_addr() -> String {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    format!("0.0.0.0:{}", port)
}

pub fn cors_allowed_origins() -> HashSet<String> {
    let mut allowed = HashSet::from([
        "http://localhost:3000".to_string(),
        "http://127.0.0.1:3000".to_string(),
        "http://localhost:8080".to_string(),
        "http://127.0.0.1:8080".to_string(),
        "https://fuelfinder-leptos-csr.vercel.app".to_string(),
    ]);

    if let Ok(origins) = std::env::var("CORS_ALLOW_ORIGINS") {
        for origin in origins.split(',').map(str::trim).filter(|o| !o.is_empty()) {
            allowed.insert(origin.to_string());
        }
    }

    allowed
}

pub async fn healthz() -> StatusCode {
    StatusCode::OK
}

pub fn build_app(app_state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            HeaderName::from_static("x-admin-password"),
        ])
        .expose_headers([
            HeaderName::from_static("x-admin-password"),
        ]);

    Router::new()
        .route("/healthz", get(healthz))
        .nest(
            "/api/v1",
            Router::new()
                .route("/healthz", get(healthz))
                .nest("/auth", auth_routes())
                .nest("/stations", stations_route())
                .nest("/commodities", commodities_route())
                .nest("/discounts", discounts_route())
                .nest("/admin", admin_routes()),
        )
        .with_state(app_state)
        .layer(cors)
}