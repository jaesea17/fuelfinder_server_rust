use axum::{
    Json,
    extract::{Path, Query, State},
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
    authentication::admin::{
        dto::{AdminStationsQuery, UpdateCommodityDiscountDto},
        model::Admins,
    },
    domain::discounts::{
        dto::AdminDiscountStatsResponse,
        service::admin_discount_stats,
    },
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
    pub commodity_id: Option<Uuid>,
    pub discount_enabled: Option<bool>,
    pub discount_percentage: Option<i32>,
    pub discount_created_count: i64,
    pub discount_redeemed_count: i64,
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
                c.id AS commodity_id,
                cd.is_enabled AS discount_enabled,
                cd.percentage AS discount_percentage,
                COALESCE(dc.created_count, 0)::BIGINT AS discount_created_count,
                COALESCE(dc.redeemed_count, 0)::BIGINT AS discount_redeemed_count,
                sub.status  AS subscription_status,
                sub.ends_at AS subscription_ends_at
            FROM stations s
            LEFT JOIN commodities c ON c.station_id = s.id
            LEFT JOIN commodity_discounts cd ON cd.commodity_id = c.id
            LEFT JOIN (
                SELECT
                    station_id,
                    COUNT(*)::BIGINT AS created_count,
                    COUNT(*) FILTER (WHERE redeemed_at IS NOT NULL)::BIGINT AS redeemed_count
                FROM discount_codes
                GROUP BY station_id
            ) dc ON dc.station_id = s.id
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

    pub async fn update_discount_config(
        State(app_state): State<AppState>,
        Path(commodity_id): Path<Uuid>,
        headers: HeaderMap,
        Json(body): Json<UpdateCommodityDiscountDto>,
    ) -> Result<impl IntoResponse, StationError> {
        if body.commodity_id != commodity_id {
            return Err(StationError::WrongCredentials(
                "commodity_id mismatch between path and payload".to_string(),
            ));
        }

        let admin_id = Self::verify_admin_request(&app_state.pool, &headers).await?;

        if body.enabled {
            let percentage = body.percentage.ok_or_else(|| {
                StationError::WrongCredentials("percentage is required when enabling discount".to_string())
            })?;

            if !(1..=10).contains(&percentage) {
                return Err(StationError::WrongCredentials(
                    "percentage must be between 1 and 10".to_string(),
                ));
            }

            sqlx::query(
                r#"
                INSERT INTO commodity_discounts (commodity_id, is_enabled, percentage, updated_by_admin, updated_at)
                VALUES ($1, TRUE, $2, $3, now())
                ON CONFLICT (commodity_id)
                DO UPDATE SET
                    is_enabled = EXCLUDED.is_enabled,
                    percentage = EXCLUDED.percentage,
                    updated_by_admin = EXCLUDED.updated_by_admin,
                    updated_at = now()
                "#,
            )
            .bind(commodity_id)
            .bind(percentage)
            .bind(admin_id)
            .execute(&app_state.pool)
            .await
            .map_err(StationError::DatabaseError)?;
        } else {
            sqlx::query(
                r#"
                INSERT INTO commodity_discounts (commodity_id, is_enabled, percentage, updated_by_admin, updated_at)
                VALUES ($1, FALSE, NULL, $2, now())
                ON CONFLICT (commodity_id)
                DO UPDATE SET
                    is_enabled = FALSE,
                    percentage = NULL,
                    updated_by_admin = EXCLUDED.updated_by_admin,
                    updated_at = now()
                "#,
            )
            .bind(commodity_id)
            .bind(admin_id)
            .execute(&app_state.pool)
            .await
            .map_err(StationError::DatabaseError)?;
        }

        Ok(StatusCode::NO_CONTENT)
    }

    pub async fn get_discount_stats(
        State(app_state): State<AppState>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, StationError> {
        Self::verify_admin_request(&app_state.pool, &headers).await?;

        let stats = admin_discount_stats(&app_state.pool)
            .await
            .map_err(StationError::DatabaseError)?;

        Ok((
            StatusCode::OK,
            Json(AdminDiscountStatsResponse {
                created_codes: stats.created_codes,
                redeemed_codes: stats.redeemed_codes,
            }),
        ))
    }
}
