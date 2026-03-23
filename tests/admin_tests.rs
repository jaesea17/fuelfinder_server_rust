mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};
use serial_test::serial;

use common::{
    call, db_pool, decode_json, mark_station_subscription_expired, request, request_with_headers,
    request_with_json, reset_db, seed_admin, station_id_by_email, test_app, test_app_with_pool,
};

async fn create_station(app: axum::Router, email: &str, station_type: &str) {
    let code = format!("REG-{}", uuid::Uuid::new_v4().simple());

    let _ = call(
        app.clone(),
        request_with_json(
            "POST",
            "/api/v1/auth/reg-code",
            json!({ "code": code, "super_password": "super-secret" }),
        ),
    )
    .await;

    let signup = call(
        app,
        request_with_json(
            "POST",
            "/api/v1/auth/signup",
            json!({
                "name": format!("{} station", station_type),
                "address": "Jabi",
                "email": email,
                "phone": "08099990000",
                "password": "station-pass",
                "latitude": 9.06,
                "longitude": 7.41,
                "code": code,
                "station_type": station_type
            }),
        ),
    )
    .await;

    assert_eq!(signup.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn admin_stations_route_exists() {
    let response = call(test_app(), request("POST", "/api/v1/admin/stations")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn admin_discount_stats_route_exists() {
    let response = call(test_app(), request("POST", "/api/v1/admin/discounts/stats")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn admin_discount_update_route_exists() {
    let response = call(
        test_app(),
        request("GET", "/api/v1/admin/discounts/550e8400-e29b-41d4-a716-446655440000"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
#[serial]
async fn admin_stations_requires_valid_password() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed admin test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool);
    let response = call(
        app,
        common::request_with_headers("GET", "/api/v1/admin/stations?filter=all", &[("x-admin-password", "wrong")]),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn admin_station_filtering_returns_active_and_expired_sets() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed admin test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool.clone());
    let active_email = format!("active-{}@example.com", uuid::Uuid::new_v4().simple());
    let expired_email = format!("expired-{}@example.com", uuid::Uuid::new_v4().simple());

    create_station(app.clone(), &active_email, "petrol").await;
    create_station(app.clone(), &expired_email, "gas").await;

    let expired_station_id = station_id_by_email(&pool, &expired_email).await;
    mark_station_subscription_expired(&pool, expired_station_id).await;

    let active_response = call(
        app.clone(),
        request_with_headers(
            "GET",
            "/api/v1/admin/stations?filter=active",
            &[("x-admin-password", "super-secret")],
        ),
    )
    .await;
    assert_eq!(active_response.status(), StatusCode::OK);
    let active_body: Value = decode_json(active_response).await;
    assert_eq!(active_body.as_array().map(Vec::len), Some(1));
    assert_eq!(active_body[0]["email"].as_str(), Some(active_email.as_str()));

    let expired_response = call(
        app.clone(),
        request_with_headers(
            "GET",
            "/api/v1/admin/stations?filter=expired",
            &[("x-admin-password", "super-secret")],
        ),
    )
    .await;
    assert_eq!(expired_response.status(), StatusCode::OK);
    let expired_body: Value = decode_json(expired_response).await;
    assert_eq!(expired_body.as_array().map(Vec::len), Some(1));
    assert_eq!(expired_body[0]["email"].as_str(), Some(expired_email.as_str()));

    let all_response = call(
        app,
        request_with_headers(
            "GET",
            "/api/v1/admin/stations?filter=all",
            &[("x-admin-password", "super-secret")],
        ),
    )
    .await;
    assert_eq!(all_response.status(), StatusCode::OK);
    let all_body: Value = decode_json(all_response).await;
    assert_eq!(all_body.as_array().map(Vec::len), Some(2));
}
