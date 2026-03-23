mod common;

use axum::http::StatusCode;
use serde_json::{Value, json};
use serial_test::serial;

use common::{
    call, commodity_id_for_station, db_pool, decode_json, request, request_with_json, reset_db,
    seed_admin, station_id_by_email, test_app, test_app_with_pool,
};

#[tokio::test]
async fn commodities_collection_route_exists() {
    let response = call(test_app(), request("POST", "/api/v1/commodities")).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
async fn commodity_update_requires_auth() {
    let response = call(
        test_app(),
        request("PATCH", "/api/v1/commodities/550e8400-e29b-41d4-a716-446655440000"),
    )
    .await;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
#[serial]
async fn commodity_update_happy_path_updates_price_and_availability() {
    let Some(pool) = db_pool().await else {
        eprintln!("Skipping DB-backed commodity test: TEST_DATABASE_URL not set");
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
                "name": "Commodity Station",
                "address": "Maitama",
                "email": email,
                "phone": "08022223333",
                "password": "station-pass",
                "latitude": 9.08,
                "longitude": 7.49,
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

    let update_response = call(
        app,
        common::request_with_headers_and_json(
            "PATCH",
            &format!("/api/v1/commodities/{commodity_id}"),
            &[("authorization", &format!("Bearer {token}"))],
            json!({ "price": 845, "is_available": true }),
        ),
    )
    .await;

    assert_eq!(update_response.status(), StatusCode::OK);
    let body: Value = decode_json(update_response).await;
    assert_eq!(body["price"].as_i64(), Some(845));
    assert_eq!(body["is_available"].as_bool(), Some(true));

    let updated_price: i32 = sqlx::query_scalar("SELECT price FROM commodities WHERE id = $1")
        .bind(commodity_id)
        .fetch_one(&pool)
        .await
        .expect("updated price should exist");

    assert_eq!(updated_price, 845);
}
