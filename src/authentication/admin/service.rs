use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use bcrypt;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    authentication::admin::{dto::AdminStationsQuery, model::Admins},
    domain::utils::errors::station_errors::StationError,
};

#[derive(Debug, Serialize, FromRow)]
pub struct StationWithSubscription {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub station_type: String,
    pub subscription_status: Option<String>,
    pub subscription_ends_at: Option<DateTime<Utc>>,
}

pub struct AdminService;

impl AdminService {
    async fn verify_admin_request(
        pool: &sqlx::PgPool,
        headers: &HeaderMap,
    ) -> Result<Uuid, StationError> {
        let password = headers
            .get("x-admin-password")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        let admin = sqlx::query_as::<_, Admins>(
            "SELECT id, role, password, created_at, updated_at FROM admins LIMIT 1",
        )
        .fetch_optional(pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let admin =
            admin.ok_or_else(|| StationError::NotFound("admin not configured".to_string()))?;

        let is_valid = bcrypt::verify(password, &admin.password)
            .map_err(|_| StationError::WrongCredentials("admin password".to_string()))?;

        if !is_valid {
            return Err(StationError::WrongCredentials(
                "invalid admin password".to_string(),
            ));
        }

        Ok(admin.id)
    }

    pub async fn get_stations(
        State(app_state): State<AppState>,
        Query(query): Query<AdminStationsQuery>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, StationError> {
        Self::verify_admin_request(&app_state.pool, &headers).await?;

        let filter = query.filter.as_deref().unwrap_or("all");

        let rows = sqlx::query_as::<_, StationWithSubscription>(
            r#"
            SELECT
                s.id,
                s.name,
                s.address,
                s.email,
                s.phone,
                s.station_type,
                sub.status  AS subscription_status,
                sub.ends_at AS subscription_ends_at
            FROM stations s
            LEFT JOIN subscriptions sub
                   ON sub.station_id = s.id AND sub.status = 'active'
            ORDER BY s.created_at DESC
            "#,
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let now = Utc::now();

        let filtered: Vec<StationWithSubscription> = match filter {
            "active" => rows
                .into_iter()
                .filter(|s| {
                    s.subscription_status.as_deref() == Some("active")
                        && s.subscription_ends_at
                            .map(|e| e > now)
                            .unwrap_or(false)
                })
                .collect(),
            "expired" => rows
                .into_iter()
                .filter(|s| {
                    s.subscription_status.is_none()
                        || s.subscription_ends_at
                            .map(|e| e <= now)
                            .unwrap_or(true)
                })
                .collect(),
            _ => rows,
        };

        Ok((StatusCode::OK, Json(filtered)))
    }
}
