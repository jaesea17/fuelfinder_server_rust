use axum::{Router, middleware::from_fn, routing::{get, post}};

use crate::{
    app_state::AppState,
    authentication::middleware::auth::authorize,
    domain::discounts::service::DiscountService,
};

pub fn discounts_route() -> Router<AppState> {
    Router::new()
        .route("/generate", post(DiscountService::generate_code))
        .route(
            "/redeem",
            post(DiscountService::redeem_code).route_layer(from_fn(authorize)),
        )
        .route(
            "/station/stats",
            get(DiscountService::station_stats).route_layer(from_fn(authorize)),
        )
}
