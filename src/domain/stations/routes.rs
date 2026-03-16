use axum::{Router, middleware::from_fn, routing::{get, patch}};

use crate::{
    app_state::AppState, authentication::middleware::auth::authorize,
    domain::stations::model::Station,
};

pub fn stations_route() -> Router<AppState> {
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
        .route("/closest", get(Station::find_closest_stations))
}
