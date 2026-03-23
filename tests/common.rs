#![allow(dead_code)]

use axum::{
    Router,
    body::{Body, to_bytes},
    http::Request,
};
use bcrypt::hash;
use chrono::NaiveDate;
use fuelfinder_server::{
    app_state::AppState,
    authentication::station::authenticate::token::service::TokenService,
    build_app,
    domain::utils::schemas::{CommoditiesResponse, StationResponse},
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use uuid::Uuid;

pub fn test_app() -> Router {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://postgres:postgres@localhost/fuelfinder_test")
        .expect("lazy pool should build");

    build_app(AppState { pool })
}

pub fn test_app_with_pool(pool: PgPool) -> Router {
    build_app(AppState { pool })
}

pub fn test_database_url() -> Option<String> {
    std::env::var("TEST_DATABASE_URL").ok()
}

pub async fn db_pool() -> Option<PgPool> {
    let url = test_database_url()?;

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("TEST_DATABASE_URL should point to a reachable Postgres database");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("test migrations should run");

    Some(pool)
}

pub async fn reset_db(pool: &PgPool) {
    sqlx::query(
        r#"
        TRUNCATE TABLE
            discount_code_generation_logs,
            discount_codes,
            commodity_discounts,
            notifications,
            subscription_reminder_logs,
            subscriptions,
            registration_codes,
            commodities,
            stations,
            admins
        RESTART IDENTITY CASCADE
        "#,
    )
    .execute(pool)
    .await
    .expect("test tables should truncate");
}

pub async fn seed_admin(pool: &PgPool, password: &str) -> Uuid {
    let password_hash = hash(password, 12).expect("admin hash should build");
    let admin_id = Uuid::new_v4();

    sqlx::query(
        r#"
        INSERT INTO admins (id, role, password)
        VALUES ($1, 'admin', $2)
        "#,
    )
    .bind(admin_id)
    .bind(password_hash)
    .execute(pool)
    .await
    .expect("admin should insert");

    admin_id
}

pub async fn seed_registration_code(pool: &PgPool, code: &str) {
    sqlx::query(
        r#"
        INSERT INTO registration_codes (code, is_valid)
        VALUES ($1, TRUE)
        "#,
    )
    .bind(code)
    .execute(pool)
    .await
    .expect("registration code should insert");
}

pub async fn station_id_by_email(pool: &PgPool, email: &str) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM stations WHERE email = $1")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("station should exist")
}

pub async fn commodity_id_for_station(pool: &PgPool, station_id: Uuid) -> Uuid {
    sqlx::query_scalar::<_, Uuid>("SELECT id FROM commodities WHERE station_id = $1 LIMIT 1")
        .bind(station_id)
        .fetch_one(pool)
        .await
        .expect("commodity should exist")
}

pub async fn mark_station_subscription_expired(pool: &PgPool, station_id: Uuid) {
    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'expired', ends_at = now() - interval '1 day'
        WHERE station_id = $1 AND status = 'active'
        "#,
    )
    .bind(station_id)
    .execute(pool)
    .await
    .expect("subscription should update");
}

pub async fn create_notification(
    pool: &PgPool,
    station_id: Uuid,
    title: &str,
    body: &str,
    kind: &str,
) -> Uuid {
    sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO notifications (station_id, title, body, kind)
        VALUES ($1, $2, $3, $4)
        RETURNING id
        "#,
    )
    .bind(station_id)
    .bind(title)
    .bind(body)
    .bind(kind)
    .fetch_one(pool)
    .await
    .expect("notification should insert")
}

pub async fn decode_json<T: DeserializeOwned>(response: axum::response::Response) -> T {
    let text = body_text(response).await;
    serde_json::from_str(&text).expect("response should be valid json")
}

pub fn request_with_json(method: &str, path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

pub fn request_with_headers_and_json(
    method: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Value,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");

    for (key, value) in headers {
        builder = builder.header(*key, *value);
    }

    builder
        .body(Body::from(body.to_string()))
        .expect("request should build")
}

pub fn request(method: &str, path: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .body(Body::empty())
        .expect("request should build")
}

pub fn request_with_auth(method: &str, path: &str, token: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .expect("request should build")
}

pub fn request_with_headers(method: &str, path: &str, headers: &[(&str, &str)]) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(path);

    for (key, value) in headers {
        builder = builder.header(*key, *value);
    }

    builder.body(Body::empty()).expect("request should build")
}

pub async fn body_text(response: axum::response::Response) -> String {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body should be readable");
    String::from_utf8(bytes.to_vec()).expect("body should be utf8")
}

pub async fn call(app: Router, request: Request<Body>) -> axum::response::Response {
    app.oneshot(request).await.expect("router call should succeed")
}

pub fn valid_token() -> String {
    unsafe {
        std::env::set_var("JWT_SECRET", "test-secret");
    }

    let created_at = NaiveDate::from_ymd_opt(2026, 1, 1)
        .expect("date")
        .and_hms_opt(0, 0, 0)
        .expect("time");

    let station = StationResponse {
        id: Uuid::new_v4(),
        name: "Test Station".to_string(),
        address: "Abuja".to_string(),
        email: "station@example.com".to_string(),
        phone: "08000000000".to_string(),
        latitude: 9.0,
        longitude: 7.0,
        role: "station".to_string(),
        station_type: "petrol".to_string(),
        created_at,
        updated_at: created_at,
        distance: Some(0.0),
        commodities: vec![CommoditiesResponse {
            id: Uuid::new_v4(),
            name: "PMS".to_string(),
            is_available: true,
            price: 700,
            station_id: Uuid::new_v4(),
            discount_enabled: Some(false),
            discount_percentage: None,
        }],
    };

    TokenService::new("test-secret")
        .create_token(station)
        .expect("token should be created")
}
