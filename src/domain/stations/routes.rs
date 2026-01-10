use axum::{Router, middleware::from_fn, routing::get};

use crate::{
    app_state::AppState, authentication::middleware::auth::authorize,
    domain::stations::station::Station,
};

pub fn stations_route() -> Router<AppState> {
    Router::new()
        .route("/", get(Station::get_stations))
        .route(
            "/dashboard",
            get(Station::get_station).route_layer(from_fn(authorize)),
        )
        .route("/closest", get(Station::find_closest_stations))
}
