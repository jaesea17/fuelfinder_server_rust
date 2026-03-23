#![forbid(clippy::unwrap_used)]

use fuelfinder_server::{
    app_state::AppState,
    build_app,
    domain::{
        subscriptions::worker::start as start_subscription_worker,
        utils::setup_tracing::setup_tracing,
    },
    listen_addr,
};

/* ========================================================== */
/*                         🦀 MAIN 🦀                         */
/* ========================================================== */

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    setup_tracing();

    let app_state = AppState::init()
        .await
        .expect("Failed to initialize database");

    tokio::spawn(start_subscription_worker(app_state.pool.clone()));

    let app = build_app(app_state);

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
