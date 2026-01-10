use axum::{body::Body, extract::Request, http::StatusCode, middleware::Next, response::Response};

use crate::authentication::{
    roles::roles::UserRole, station::authenticate::token::service::Claims,
};

pub async fn authorize_role(
    required_role: Vec<String>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 1. Get the user from the request extensions
    // (This assumes you have an Auth middleware that ran before this)
    let user = req
        .extensions()
        .get::<Claims>()
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // 2. Check if the user's role matches
    if required_role.contains(&user.station_res.role) {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::FORBIDDEN)
    }
}
