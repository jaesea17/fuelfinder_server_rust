use axum::{extract::Request, middleware::Next, response::Response};

use crate::{
    authentication::station::authenticate::token::service::TokenService,
    domain::utils::errors::station_errors::StationError,
};

pub async fn authorize(mut request: Request, next: Next) -> Result<Response, StationError> {
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let access_token = match auth_header {
        Some(value) if value.starts_with("Bearer ") => {
            let access_token = value.trim_start_matches("Bearer ").trim();
            access_token.to_string()
        }
        _ => {
            return Err(StationError::WrongCredentials(String::from(
                "jwt not present",
            )));
        }
    };

    let secret = std::env::var("JWT_SECRET").expect("JWT_SECRET not set");
    let token_service = TokenService::new(&secret);
    let decoded = token_service.decode(access_token);
    let claims = match decoded {
        Ok(decoded) => decoded.claims,
        Err(_) => return Err(StationError::WrongCredentials(String::from("token"))),
    };
    //TODO retrieve station by id(columns role, is_logged_in).
    //If token expired, id not found or false to is_logged_in throw StationError::WrongCredentials()
    //
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}
