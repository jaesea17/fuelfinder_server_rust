mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};
use serial_test::serial;
use uuid::Uuid;

use common::{
    body_text, call, create_notification, db_pool, decode_json, mark_station_subscription_expired,
    request, request_with_auth, request_with_json, reset_db, seed_admin,
    test_app, test_app_with_pool, valid_token,
};

async fn create_station_and_signin(
    app: axum::Router,
    email: &str,
    station_type: &str,
) -> (Uuid, String) {
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
        app.clone(),
        request_with_json(
            "POST",
            "/api/v1/auth/signup",
            json!({
                "name": format!("{} station", station_type),
                "address": "Central Area",
                "email": email,
                "phone": "08000001111",
                "password": "station-pass",
                "latitude": 9.08,
                "longitude": 7.48,
                "code": code,
                "station_type": station_type
            }),
        ),
    )
    .await;
    assert_eq!(signup.status(), StatusCode::CREATED);

    let signup_body: Value = decode_json(signup).await;
    let station_id = uuid::Uuid::parse_str(signup_body["id"].as_str().unwrap()).unwrap();

    let signin = call(
        app,
        request_with_json(
            "POST",
            "/api/v1/auth/signin",
            json!({
                "email": email,
                "password": "station-pass",
                "station_type": station_type
            }),
        ),
    )
    .await;
    assert_eq!(signin.status(), StatusCode::OK);

    let signin_body: Value = decode_json(signin).await;
    let token = signin_body["access_token"].as_str().unwrap().to_string();
    (station_id, token)
}

#[tokio::test]
async fn root_healthz_returns_ok() {
    let response = call(test_app(), request("GET", "/healthz")).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn nested_healthz_returns_ok() {
    let response = call(test_app(), request("GET", "/api/v1/healthz")).await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn station_dashboard_requires_bearer_token() {
    let response = call(test_app(), request("GET", "/api/v1/stations/dashboard")).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    let body = body_text(response).await;
    assert!(body.contains("jwt not present"));
}

#[tokio::test]
async fn station_dashboard_rejects_invalid_token() {
    let response = call(
        test_app(),
        request_with_auth("GET", "/api/v1/stations/dashboard", "not-a-token"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_text(response).await;
    assert!(body.contains("token"));
}

#[tokio::test]
async fn station_dashboard_accepts_valid_token_then_reaches_handler() {
    let response = call(
        test_app(),
        request_with_auth("GET", "/api/v1/stations/dashboard", &valid_token()),
    )
    .await;

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn dashboard_notifications_require_auth() {
    let response = call(
        test_app(),
        request("GET", "/api/v1/stations/dashboard/notifications"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn mark_notification_read_requires_auth() {
    let response = call(
        test_app(),
        request(
            "PATCH",
            "/api/v1/stations/dashboard/notifications/550e8400-e29b-41d4-a716-446655440000/read",
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn closest_endpoint_rejects_out_of_boundary_coordinates_before_db_access() {
    let response = call(
        test_app(),
        request(
            "GET",
            "/api/v1/stations/closest?latitude=0.0&longitude=0.0&station_type=petrol",
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let body = body_text(response).await;
    assert!(body.contains("outside Abuja service area"));
}

#[tokio::test]
#[serial]
async fn dashboard_notifications_returns_seeded_notifications() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed stations test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool.clone());
    let email = format!("{}@example.com", uuid::Uuid::new_v4().simple());
    let (station_id, token) = create_station_and_signin(app.clone(), &email, "petrol").await;

    create_notification(
        &pool,
        station_id,
        "Subscription reminder",
        "Please renew soon",
        "subscription",
    )
    .await;

    let response = call(
        app,
        request_with_auth("GET", "/api/v1/stations/dashboard/notifications", &token),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = decode_json(response).await;
    assert_eq!(body.as_array().map(Vec::len), Some(1));
    assert_eq!(body[0]["title"].as_str(), Some("Subscription reminder"));
    assert_eq!(body[0]["is_read"].as_bool(), Some(false));
}

#[tokio::test]
#[serial]
async fn mark_notification_read_updates_record() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed stations test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool.clone());
    let email = format!("{}@example.com", uuid::Uuid::new_v4().simple());
    let (station_id, token) = create_station_and_signin(app.clone(), &email, "petrol").await;

    let notification_id = create_notification(
        &pool,
        station_id,
        "Expiry",
        "Expired subscription",
        "subscription",
    )
    .await;

    let response = call(
        app,
        request_with_auth(
            "PATCH",
            &format!("/api/v1/stations/dashboard/notifications/{notification_id}/read"),
            &token,
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::NO_CONTENT);

    let is_read: bool = sqlx::query_scalar("SELECT is_read FROM notifications WHERE id = $1")
        .bind(notification_id)
        .fetch_one(&pool)
        .await
        .expect("notification should exist");

    assert!(is_read);
}

#[tokio::test]
#[serial]
async fn expired_subscription_signin_creates_notification() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed stations test: TEST_DATABASE_URL not set");
        return;
    };

    reset_db(&pool).await;
    seed_admin(&pool, "super-secret").await;

    let app = test_app_with_pool(pool.clone());
    let email = format!("{}@example.com", uuid::Uuid::new_v4().simple());

    let (station_id, _token) = create_station_and_signin(app.clone(), &email, "gas").await;
    mark_station_subscription_expired(&pool, station_id).await;

    let signin_response = call(
        app,
        request_with_json(
            "POST",
            "/api/v1/auth/signin",
            json!({
                "email": email,
                "password": "station-pass",
                "station_type": "gas"
            }),
        ),
    )
    .await;

    assert_eq!(signin_response.status(), StatusCode::UNAUTHORIZED);

    let notification_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM notifications WHERE station_id = $1 AND kind = 'subscription'",
    )
    .bind(station_id)
    .fetch_one(&pool)
    .await
    .expect("notification count query should succeed");

    assert_eq!(notification_count, 1);
}
