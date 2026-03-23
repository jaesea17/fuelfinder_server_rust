mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};
use serial_test::serial;

use common::{
    call, commodity_id_for_station, db_pool, decode_json, request, request_with_json, reset_db,
    seed_admin, station_id_by_email, test_app, test_app_with_pool,
};

#[tokio::test]
async fn discount_generate_route_exists() {
    let response = call(test_app(), request("GET", "/api/v1/discounts/generate")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn discount_redeem_requires_auth() {
    let response = call(test_app(), request("POST", "/api/v1/discounts/redeem")).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn station_discount_stats_requires_auth() {
    let response = call(test_app(), request("GET", "/api/v1/discounts/station/stats")).await;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn admin_can_enable_discount_generate_code_redeem_and_view_stats() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed discount test: TEST_DATABASE_URL not set");
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
                "name": "Discount Station",
                "address": "Utako",
                "email": email,
                "phone": "08011112222",
                "password": "station-pass",
                "latitude": 9.10,
                "longitude": 7.45,
                "code": code,
                "station_type": "petrol"
            }),
        ),
    )
    .await;

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
    let signin_body: Value = decode_json(signin_response).await;
    let token = signin_body["access_token"].as_str().unwrap().to_string();

    let station_id = station_id_by_email(&pool, &email).await;
    let commodity_id = commodity_id_for_station(&pool, station_id).await;

    let enable_discount_response = call(
        app.clone(),
        common::request_with_headers_and_json(
            "PATCH",
            &format!("/api/v1/admin/discounts/{commodity_id}"),
            &[("x-admin-password", "super-secret")],
            json!({
                "commodity_id": commodity_id,
                "enabled": true,
                "percentage": 10
            }),
        ),
    )
    .await;
    assert_eq!(enable_discount_response.status(), StatusCode::NO_CONTENT);

    let generate_response = call(
        app.clone(),
        common::request_with_headers_and_json(
            "POST",
            "/api/v1/discounts/generate",
            &[("x-forwarded-for", "203.0.113.99")],
            json!({ "station_id": station_id }),
        ),
    )
    .await;
    assert_eq!(generate_response.status(), StatusCode::CREATED);

    let generate_body: Value = decode_json(generate_response).await;
    let discount_code = generate_body["code"].as_str().unwrap().to_string();
    assert_eq!(generate_body["discount_percentage"].as_i64(), Some(10));

    let redeem_response = call(
        app.clone(),
        common::request_with_headers_and_json(
            "POST",
            "/api/v1/discounts/redeem",
            &[("authorization", &format!("Bearer {token}"))],
            json!({ "code": discount_code }),
        ),
    )
    .await;
    assert_eq!(redeem_response.status(), StatusCode::OK);

    let redeem_body: Value = decode_json(redeem_response).await;
    assert_eq!(redeem_body["message"].as_str(), Some("code redeemed successfully"));
    assert_eq!(redeem_body["is_expired"].as_bool(), Some(false));

    let station_stats_response = call(
        app.clone(),
        common::request_with_auth("GET", "/api/v1/discounts/station/stats", &token),
    )
    .await;
    assert_eq!(station_stats_response.status(), StatusCode::OK);

    let station_stats_body: Value = decode_json(station_stats_response).await;
    assert_eq!(station_stats_body["redeemed_codes"].as_i64(), Some(1));

    let admin_stats_response = call(
        app,
        common::request_with_headers(
            "GET",
            "/api/v1/admin/discounts/stats",
            &[("x-admin-password", "super-secret")],
        ),
    )
    .await;
    assert_eq!(admin_stats_response.status(), StatusCode::OK);

    let admin_stats_body: Value = decode_json(admin_stats_response).await;
    assert_eq!(admin_stats_body["created_codes"].as_i64(), Some(1));
    assert_eq!(admin_stats_body["redeemed_codes"].as_i64(), Some(1));
}
