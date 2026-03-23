mod common;

use std::time::Duration;

use axum::http::StatusCode;
use fuelfinder_server::domain::utils::rate_limiter::RateLimiter;

use common::{body_text, call, request_with_headers, test_app};

#[test]
fn limiter_blocks_requests_after_limit() {
    let limiter = RateLimiter::new(2, Duration::from_secs(60));

    assert!(limiter.is_allowed("1.2.3.4"));
    assert!(limiter.is_allowed("1.2.3.4"));
    assert!(!limiter.is_allowed("1.2.3.4"));
}

#[test]
fn limiter_resets_after_window_expires() {
    let limiter = RateLimiter::new(1, Duration::from_millis(20));

    assert!(limiter.is_allowed("1.2.3.4"));
    assert!(!limiter.is_allowed("1.2.3.4"));

    std::thread::sleep(Duration::from_millis(25));

    assert!(limiter.is_allowed("1.2.3.4"));
}

#[tokio::test]
async fn closest_endpoint_rate_limits_same_forwarded_ip() {
    let app = test_app();

    for _ in 0..10 {
        let response = call(
            app.clone(),
            request_with_headers(
                "GET",
                "/api/v1/stations/closest?latitude=0.0&longitude=0.0&station_type=petrol",
                &[("x-forwarded-for", "203.0.113.10")],
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let response = call(
        app,
        request_with_headers(
            "GET",
            "/api/v1/stations/closest?latitude=0.0&longitude=0.0&station_type=petrol",
            &[("x-forwarded-for", "203.0.113.10")],
        ),
    )
    .await;

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    let body = body_text(response).await;
    assert!(body.contains("Too many requests"));
}

#[tokio::test]
async fn closest_endpoint_tracks_different_ips_independently() {
    let app = test_app();

    for _ in 0..10 {
        let response = call(
            app.clone(),
            request_with_headers(
                "GET",
                "/api/v1/stations/closest?latitude=0.0&longitude=0.0&station_type=petrol",
                &[("x-forwarded-for", "198.51.100.1")],
            ),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    let other_ip_response = call(
        app,
        request_with_headers(
            "GET",
            "/api/v1/stations/closest?latitude=0.0&longitude=0.0&station_type=petrol",
            &[("x-forwarded-for", "198.51.100.2")],
        ),
    )
    .await;

    assert_eq!(other_ip_response.status(), StatusCode::UNAUTHORIZED);
}
