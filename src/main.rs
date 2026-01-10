#![forbid(clippy::unwrap_used)]

use axum::Router;
use http::header::{AUTHORIZATION, CONTENT_TYPE};
use tower_http::cors::{Any, CorsLayer};

mod app_state;
mod authentication;
mod domain;

use crate::{
    app_state::AppState,
    authentication::station::authenticate::routes::auth_routes,
    domain::{
        commodities::routes::commodities_route, stations::routes::stations_route,
        utils::setup_tracing::setup_tracing,
    },
};
// use crate::utils::setup_tracing::setup_tracing;

/* ========================================================== */
/*                         🦀 MAIN 🦀                         */
/* ========================================================== */

const PORT_8000: &str = "0.0.0.0:8000";

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    setup_tracing();

    let cors = CorsLayer::new()
        // .allow_origin("http://127.0.0.1:3000".parse::<HeaderValue>().unwrap())
        .allow_origin(tower_http::cors::AllowOrigin::mirror_request())
        .allow_methods(Any)
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let app_state = AppState::init()
        .await
        .expect("Failed to initialize database");

    let app = Router::new()
        .nest(
            "/api/v1",
            Router::new()
                .nest("/auth", auth_routes())
                .nest("/stations", stations_route())
                .nest("/commodities", commodities_route()),
        )
        .with_state(app_state)
        .layer(cors);
    // let app = Router::new().merge(books_routes()).with_state(app_state);

    let listener = tokio::net::TcpListener::bind(PORT_8000)
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
