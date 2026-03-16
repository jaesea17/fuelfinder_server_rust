use axum::{Router, routing::{get, patch}};

use crate::{app_state::AppState, authentication::admin::service::AdminService};

pub fn admin_routes() -> Router<AppState> {
    Router::new()
        .route("/stations", get(AdminService::get_stations))
        .route("/discounts/stats", get(AdminService::get_discount_stats))
        .route(
            "/discounts/:commodity_id",
            patch(AdminService::update_discount_config),
        )
}
