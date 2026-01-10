use crate::app_state::AppState;
use crate::authentication::station::authenticate::dto::StationWithCommodity;
use axum::Router;
use axum::routing::post;

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/signin", post(StationWithCommodity::signin))
        .route("/signup", post(StationWithCommodity::signup))
}
