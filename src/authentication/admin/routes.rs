use axum::{Router, routing::get};

use crate::{app_state::AppState, authentication::admin::service::AdminService};

pub fn admin_routes() -> Router<AppState> {
    Router::new().route("/stations", get(AdminService::get_stations))
}
