#![forbid(clippy::unwrap_used)]

use axum::Router;
use http::{HeaderName, header::{AUTHORIZATION, CONTENT_TYPE}};
use std::collections::HashSet;
use tower_http::cors::{Any, CorsLayer};

mod app_state;
mod authentication;
mod domain;

use crate::{
    app_state::AppState,
    authentication::{
        admin::routes::admin_routes,
        station::authenticate::routes::auth_routes,
    },
    domain::{
        commodities::routes::commodities_route, discounts::routes::discounts_route,
        stations::routes::stations_route,
        subscriptions::worker::start as start_subscription_worker,
        utils::setup_tracing::setup_tracing,
    },
};
// use crate::utils::setup_tracing::setup_tracing;

/* ========================================================== */
/*                         🦀 MAIN 🦀                         */
/* ========================================================== */

// Allow the listen address/port to be overridden via the `PORT` env var
// (used by Docker Compose, tests, etc). Defaults to 8000.
fn listen_addr() -> String {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    format!("0.0.0.0:{}", port)
}

fn cors_allowed_origins() -> HashSet<String> {
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

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    setup_tracing();

    let _allowed_origins = cors_allowed_origins();

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

    let app_state = AppState::init()
        .await
        .expect("Failed to initialize database");

    tokio::spawn(start_subscription_worker(app_state.pool.clone()));

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .nest("/auth", auth_routes())
                .nest("/stations", stations_route())
                .nest("/commodities", commodities_route())
                .nest("/discounts", discounts_route())
                .nest("/admin", admin_routes()),
        )
        .with_state(app_state)
        .layer(cors);
    // let app = Router::new().merge(books_routes()).with_state(app_state);

    let addr = listen_addr();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    tracing::debug!(
        "listening on {}",
        listener.local_addr().expect("Failed to get local address")
    );

    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
