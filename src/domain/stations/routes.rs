use axum::{Router, middleware::{from_fn, from_fn_with_state}, routing::{get, patch}};
use std::time::Duration;

use crate::{
    app_state::AppState, authentication::middleware::auth::authorize,
    domain::{
        stations::model::Station,
        utils::rate_limiter::{RateLimiter, closest_stations_rate_limit},
    },
};

/// 10 requests per 60 seconds per IP on the /closest endpoint.
const CLOSEST_MAX_REQUESTS: u32 = 10;
const CLOSEST_WINDOW_SECS: u64 = 60;

pub fn stations_route() -> Router<AppState> {
    let rate_limiter = RateLimiter::new(
        CLOSEST_MAX_REQUESTS,
        Duration::from_secs(CLOSEST_WINDOW_SECS),
    );

    // Periodic cleanup of stale IP entries.
    let cleanup_limiter = rate_limiter.clone();
    tokio::spawn(async move {
        let interval = Duration::from_secs(CLOSEST_WINDOW_SECS * 2);
        loop {
            tokio::time::sleep(interval).await;
            cleanup_limiter.cleanup();
        }
    });

    Router::new()
        .route("/", get(Station::get_stations))
        .route(
            "/dashboard",
            get(Station::get_station).route_layer(from_fn(authorize)),
        )
        .route(
            "/dashboard/notifications",
            get(Station::get_dashboard_notifications).route_layer(from_fn(authorize)),
        )
        .route(
            "/dashboard/notifications/{notification_id}/read",
            patch(Station::mark_dashboard_notification_read).route_layer(from_fn(authorize)),
        )
        .route(
            "/closest",
            get(Station::find_closest_stations)
                .route_layer(from_fn_with_state(rate_limiter, closest_stations_rate_limit)),
        )
}
