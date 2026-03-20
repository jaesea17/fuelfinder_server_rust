use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use dashmap::DashMap;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};

// ─── data stored per IP ─────────────────────────────────────────────────────

struct WindowState {
    count:        u32,
    window_start: Instant,
}

// ─── RateLimiter ─────────────────────────────────────────────────────────────

struct RateLimiterInner {
    map:          DashMap<String, WindowState>,
    max_requests: u32,
    window:       Duration,
}

/// A cheaply-clonable, thread-safe fixed-window rate limiter keyed by IP.
#[derive(Clone)]
pub struct RateLimiter(Arc<RateLimiterInner>);

impl RateLimiter {
    /// Create a new limiter that allows `max_requests` per `window` per IP.
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self(Arc::new(RateLimiterInner {
            map: DashMap::new(),
            max_requests,
            window,
        }))
    }

    /// Returns `true` if the request from this `key` should be allowed.
    pub fn is_allowed(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut entry = self.0.map.entry(key.to_string()).or_insert_with(|| WindowState {
            count: 0,
            window_start: now,
        });

        if now.duration_since(entry.window_start) >= self.0.window {
            // New window — reset.
            entry.count = 1;
            entry.window_start = now;
            true
        } else if entry.count < self.0.max_requests {
            entry.count += 1;
            true
        } else {
            false
        }
    }

    /// Drop entries whose window started more than 2× ago — call periodically.
    pub fn cleanup(&self) {
        let now = Instant::now();
        let window = self.0.window;
        self.0
            .map
            .retain(|_, ws| now.duration_since(ws.window_start) < window * 2);
    }
}

// ─── IP extraction ───────────────────────────────────────────────────────────

fn extract_ip(req: &Request<Body>) -> String {
    // Respect reverse-proxy headers first.
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            req.extensions()
                .get::<axum::extract::ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

// ─── Axum middleware ──────────────────────────────────────────────────────────

/// Axum `from_fn_with_state` middleware that rate-limits the `/closest` endpoint.
///
/// Configured to allow **10 requests per minute** per IP.  Requests that exceed
/// that budget receive a `429 Too Many Requests` JSON response.
pub async fn closest_stations_rate_limit(
    State(limiter): State<RateLimiter>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let ip = extract_ip(&req);

    if limiter.is_allowed(&ip) {
        next.run(req).await
    } else {
        (
            StatusCode::TOO_MANY_REQUESTS,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":"Too many requests. Please wait a moment before searching again."}"#,
        )
            .into_response()
    }
}
