mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};
use serial_test::serial;

use common::{
    call, db_pool, decode_json, request, request_with_json, reset_db, seed_admin,
    station_id_by_email, test_app, test_app_with_pool,
};

#[tokio::test]
async fn signin_route_exists_and_rejects_get() {
    let response = call(test_app(), request("GET", "/api/v1/auth/signin")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn signup_route_exists_and_rejects_get() {
    let response = call(test_app(), request("GET", "/api/v1/auth/signup")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn reg_code_route_exists_and_rejects_get() {
    let response = call(test_app(), request("GET", "/api/v1/auth/reg-code")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn renew_subscription_route_exists_and_rejects_get() {
    let response = call(test_app(), request("GET", "/api/v1/auth/subscriptions/renew")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
#[serial]
async fn reg_code_requires_valid_admin_password() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed auth test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool);
    let response = call(
        app,
        request_with_json(
            "POST",
            "/api/v1/auth/reg-code",
            json!({ "code": "REG-001", "super_password": "wrong-password" }),
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn signup_signin_and_dashboard_happy_path() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed auth test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool.clone());
    let code = format!("REG-{}", uuid::Uuid::new_v4().simple());
    let email = format!("{}@example.com", uuid::Uuid::new_v4().simple());

    let reg_code_response = call(
        app.clone(),
        request_with_json(
            "POST",
            "/api/v1/auth/reg-code",
            json!({ "code": code, "super_password": "super-secret" }),
        ),
    )
    .await;
    assert_eq!(reg_code_response.status(), StatusCode::OK);

    let signup_response = call(
        app.clone(),
        request_with_json(
            "POST",
            "/api/v1/auth/signup",
            json!({
                "name": "North Station",
                "address": "Wuse Zone 2",
                "email": email,
                "phone": "08012345678",
                "password": "station-pass",
                "latitude": 9.0765,
                "longitude": 7.3986,
                "code": code,
                "station_type": "petrol"
            }),
        ),
    )
    .await;
    assert_eq!(signup_response.status(), StatusCode::CREATED);

    let signup_body: Value = decode_json(signup_response).await;
    assert_eq!(signup_body["email"].as_str(), Some(email.as_str()));
    assert_eq!(signup_body["commodities"].as_array().map(Vec::len), Some(1));

    let signin_response = call(
        app.clone(),
        request_with_json(
            "POST",
            "/api/v1/auth/signin",
            json!({
                "email": email,
                "password": "station-pass",
                "station_type": "petrol"
            }),
        ),
    )
    .await;
    assert_eq!(signin_response.status(), StatusCode::OK);

    let signin_body: Value = decode_json(signin_response).await;
    let token = signin_body["access_token"]
        .as_str()
        .expect("signin should return token")
        .to_string();

    let dashboard_response = call(
        app,
        common::request_with_auth("GET", "/api/v1/stations/dashboard", &token),
    )
    .await;
    assert_eq!(dashboard_response.status(), StatusCode::OK);

    let dashboard_body: Value = decode_json(dashboard_response).await;
    assert_eq!(dashboard_body["email"].as_str(), Some(email.as_str()));
    assert_eq!(dashboard_body["station_type"].as_str(), Some("petrol"));

    let station_id = station_id_by_email(&pool, &email).await;
    let subscription_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscriptions WHERE station_id = $1 AND status = 'active'",
    )
    .bind(station_id)
    .fetch_one(&pool)
    .await
    .expect("subscription query should succeed");

    assert_eq!(subscription_count, 1);
}

#[tokio::test]
#[serial]
async fn renew_subscription_creates_new_active_subscription() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed auth test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool.clone());
    let code = format!("REG-{}", uuid::Uuid::new_v4().simple());
    let email = format!("{}@example.com", uuid::Uuid::new_v4().simple());

    call(
        app.clone(),
        request_with_json(
            "POST",
            "/api/v1/auth/reg-code",
            json!({ "code": code, "super_password": "super-secret" }),
        ),
    )
    .await;

    call(
        app.clone(),
        request_with_json(
            "POST",
            "/api/v1/auth/signup",
            json!({
                "name": "Renew Station",
                "address": "Garki",
                "email": email,
                "phone": "08087654321",
                "password": "station-pass",
                "latitude": 9.05,
                "longitude": 7.47,
                "code": code,
                "station_type": "gas"
            }),
        ),
    )
    .await;

    let station_id = station_id_by_email(&pool, &email).await;

    let renew_response = call(
        app,
        request_with_json(
            "POST",
            "/api/v1/auth/subscriptions/renew",
            json!({
                "station_id": station_id,
                "days": 15,
                "super_password": "super-secret"
            }),
        ),
    )
    .await;

    assert_eq!(renew_response.status(), StatusCode::OK);

    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscriptions WHERE station_id = $1 AND status = 'active'",
    )
    .bind(station_id)
    .fetch_one(&pool)
    .await
    .expect("active subscription query should succeed");

    let expired_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM subscriptions WHERE station_id = $1 AND status = 'expired'",
    )
    .bind(station_id)
    .fetch_one(&pool)
    .await
    .expect("expired subscription query should succeed");

    assert_eq!(active_count, 1);
    assert_eq!(expired_count, 1);
}
