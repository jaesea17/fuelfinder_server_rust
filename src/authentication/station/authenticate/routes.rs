use crate::app_state::AppState;
use crate::authentication::station::authenticate::service::Authentication;
use axum::Router;
use axum::routing::post;

pub fn auth_routes() -> Router<AppState> {
    Router::new()
        .route("/signin", post(Authentication::signin))
        .route("/signup", post(Authentication::signup))
        .route("/reg-code", post(Authentication::create_reg_code))
    .route("/subscriptions/renew", post(Authentication::renew_subscription))
}
