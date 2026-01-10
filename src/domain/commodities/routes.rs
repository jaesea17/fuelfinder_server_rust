use axum::{
    Router,
    middleware::from_fn,
    routing::{get, patch},
};

use crate::{
    app_state::AppState,
    authentication::middleware::{auth::authorize, authorize_role::authorize_role},
    domain::commodities::commodity::Commodity,
};

pub fn commodities_route() -> Router<AppState> {
    Router::new()
        .route("/", get(Commodity::get_commodities))
        .route(
            "/{id}",
            patch(Commodity::update_commodity)
                // .route_layer(from_fn(|req, next| {
                //     authorize_role(vec!["station".to_string()], req, next)
                // }))
                .route_layer(from_fn(authorize)),
        )
}
