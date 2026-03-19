use axum::{
    Json,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    authentication::station::authenticate::token::service::Claims,
    domain::{
        discounts::{
            dto::{
                DiscountCodeResponse, GenerateDiscountCodeDto, RedeemDiscountCodeDto,
                RedeemDiscountCodeResponse, StationDiscountStatsResponse,
            },
            model::{AdminDiscountStats, DiscountCode, StationDiscountStats},
        },
        utils::errors::station_errors::StationError,
    },
};

pub struct DiscountService;

const MAX_CODES_PER_IP_PER_STATION_PER_DAY: i64 = 3;
const MAX_CODE_GENERATE_RETRY: i64 = 8;

fn station_code_prefix(station_name: &str) -> String {
    let mut chars = station_name
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .collect::<Vec<char>>();

    while chars.len() < 2 {
        chars.push('X');
    }

    chars.into_iter().take(2).collect()
}

fn station_type_suffix(station_type: &str) -> char {
    if station_type.to_ascii_lowercase().contains("petrol") {
        'P'
    } else {
        'G'
    }
}

fn generate_candidate_code(station_name: &str, station_type: &str) -> String {
    let prefix = station_code_prefix(station_name);
    let suffix = station_type_suffix(station_type);
    let random_body = Uuid::new_v4()
        .simple()
        .to_string()
        .chars()
        .take(6)
        .collect::<String>()
        .to_ascii_uppercase();

    format!("{prefix}{random_body}{suffix}")
}

fn extract_request_ip(headers: &HeaderMap) -> Option<String> {
    let forwarded_for = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.split(',').next())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string);

    if forwarded_for.is_some() {
        return forwarded_for;
    }

    headers
        .get("cf-connecting-ip")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string)
        .or_else(|| {
            headers
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToString::to_string)
        })
}

impl DiscountService {
    pub async fn generate_code(
        State(app_state): State<AppState>,
        headers: HeaderMap,
        Json(body): Json<GenerateDiscountCodeDto>,
    ) -> Result<(StatusCode, Json<DiscountCodeResponse>), StationError> {
        let ip = extract_request_ip(&headers)
            .ok_or_else(|| StationError::WrongCredentials("unable to determine caller ip".to_string()))?;

        let station = sqlx::query_as::<_, (Uuid, String, String)>(
            r#"
            SELECT id, name, station_type
            FROM stations
            WHERE id = $1
            "#,
        )
        .bind(body.station_id)
        .fetch_optional(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?
        .ok_or_else(|| StationError::NotFound(body.station_id.to_string()))?;

        let commodity = sqlx::query_as::<_, (Uuid, i32, bool, Option<i32>)>(
            r#"
            SELECT
                c.id,
                c.price,
                COALESCE(cd.is_enabled, FALSE) AS is_enabled,
                cd.percentage
            FROM commodities c
            LEFT JOIN commodity_discounts cd ON cd.commodity_id = c.id
            WHERE c.station_id = $1
            LIMIT 1
            "#,
        )
        .bind(body.station_id)
        .fetch_optional(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?
        .ok_or_else(|| StationError::NotFound("station commodity not found".to_string()))?;

        let (_station_id, station_name, station_type) = station;
        let (commodity_id, original_price, is_enabled, percentage) = commodity;

        if !is_enabled {
            return Err(StationError::WrongCredentials(
                "discount is not enabled for this station".to_string(),
            ));
        }

        let discount_percentage = percentage.ok_or_else(|| {
            StationError::WrongCredentials("discount percentage is not configured".to_string())
        })?;

        if !(1..=10).contains(&discount_percentage) {
            return Err(StationError::WrongCredentials(
                "discount percentage must be between 1 and 10".to_string(),
            ));
        }

        let generated_today: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM discount_code_generation_logs
            WHERE ip_address = $1
              AND station_id = $2
              AND created_at::date = now()::date
            "#,
        )
        .bind(&ip)
        .bind(body.station_id)
        .fetch_one(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        if generated_today >= MAX_CODES_PER_IP_PER_STATION_PER_DAY {
            return Err(StationError::WrongCredentials(
                "daily discount code limit reached for this station".to_string(),
            ));
        }

        let created_at = Utc::now();
        let expires_at = created_at + Duration::hours(24);
        let discounted_price = (original_price * (100 - discount_percentage)) / 100;

        let mut inserted: Option<DiscountCode> = None;

        for _ in 0..MAX_CODE_GENERATE_RETRY {
            let candidate_code = generate_candidate_code(&station_name, &station_type);

            let maybe_code = sqlx::query_as::<_, DiscountCode>(
                r#"
                INSERT INTO discount_codes (
                    code,
                    station_id,
                    commodity_id,
                    created_price,
                    discount_percentage,
                    discounted_price,
                    created_at,
                    expires_at
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                ON CONFLICT (code) DO NOTHING
                RETURNING
                    id,
                    code,
                    station_id,
                    commodity_id,
                    created_price,
                    discount_percentage,
                    discounted_price,
                    created_at,
                    expires_at,
                    redeemed_at,
                    redeemed_by_station_id
                "#,
            )
            .bind(candidate_code)
            .bind(body.station_id)
            .bind(commodity_id)
            .bind(original_price)
            .bind(discount_percentage)
            .bind(discounted_price)
            .bind(created_at)
            .bind(expires_at)
            .fetch_optional(&app_state.pool)
            .await
            .map_err(StationError::DatabaseError)?;

            if let Some(code) = maybe_code {
                inserted = Some(code);
                break;
            }
        }

        let inserted = inserted.ok_or_else(|| {
            StationError::WrongCredentials("failed to generate unique discount code".to_string())
        })?;

        sqlx::query(
            r#"
            INSERT INTO discount_code_generation_logs (code_id, station_id, ip_address)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(inserted.id)
        .bind(body.station_id)
        .bind(ip)
        .execute(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        Ok((
            StatusCode::CREATED,
            Json(DiscountCodeResponse {
                code: inserted.code,
                created_at: inserted.created_at,
                expires_at: inserted.expires_at,
                discount_percentage: inserted.discount_percentage,
                original_price: inserted.created_price,
                discounted_price: inserted.discounted_price,
                is_expired: Utc::now() >= inserted.expires_at,
            }),
        ))
    }

    pub async fn redeem_code(
        State(app_state): State<AppState>,
        Extension(claims): Extension<Claims>,
        Json(body): Json<RedeemDiscountCodeDto>,
    ) -> Result<(StatusCode, Json<RedeemDiscountCodeResponse>), StationError> {
        let station_id = claims.station_res.id;

        let mut tx = app_state
            .pool
            .begin()
            .await
            .map_err(StationError::DatabaseError)?;

        let code = sqlx::query_as::<_, DiscountCode>(
            r#"
            SELECT
                id,
                code,
                station_id,
                commodity_id,
                created_price,
                discount_percentage,
                discounted_price,
                created_at,
                expires_at,
                redeemed_at,
                redeemed_by_station_id
            FROM discount_codes
            WHERE code = $1
            FOR UPDATE
            "#,
        )
        .bind(body.code.trim().to_uppercase())
        .fetch_optional(&mut *tx)
        .await
        .map_err(StationError::DatabaseError)?
        .ok_or_else(|| StationError::NotFound("discount code not found".to_string()))?;

        if code.station_id != station_id {
            return Err(StationError::WrongCredentials(
                "discount code does not belong to your station".to_string(),
            ));
        }

        if code.redeemed_at.is_some() {
            tx.commit().await.map_err(StationError::DatabaseError)?;
            return Ok((
                StatusCode::OK,
                Json(RedeemDiscountCodeResponse {
                    message: "code already redeemed".to_string(),
                    code: Some(code.code),
                    created_at: Some(code.created_at),
                    expires_at: Some(code.expires_at),
                    discount_percentage: Some(code.discount_percentage),
                    discounted_price: Some(code.discounted_price),
                    is_expired: Utc::now() >= code.expires_at,
                }),
            ));
        }

        let is_expired = Utc::now() >= code.expires_at;

        if is_expired {
            tx.commit().await.map_err(StationError::DatabaseError)?;
            return Ok((
                StatusCode::OK,
                Json(RedeemDiscountCodeResponse {
                    message: "code is expired".to_string(),
                    code: Some(code.code),
                    created_at: Some(code.created_at),
                    expires_at: Some(code.expires_at),
                    discount_percentage: Some(code.discount_percentage),
                    discounted_price: Some(code.discounted_price),
                    is_expired: true,
                }),
            ));
        }

        let redeemed_at = Utc::now();

        sqlx::query(
            r#"
            UPDATE discount_codes
            SET redeemed_at = $1,
                redeemed_by_station_id = $2
            WHERE id = $3
            "#,
        )
        .bind(redeemed_at)
        .bind(station_id)
        .bind(code.id)
        .execute(&mut *tx)
        .await
        .map_err(StationError::DatabaseError)?;

        tx.commit().await.map_err(StationError::DatabaseError)?;

        Ok((
            StatusCode::OK,
            Json(RedeemDiscountCodeResponse {
                message: "code redeemed successfully".to_string(),
                code: Some(code.code),
                created_at: Some(code.created_at),
                expires_at: Some(code.expires_at),
                discount_percentage: Some(code.discount_percentage),
                discounted_price: Some(code.discounted_price),
                is_expired: false,
            }),
        ))
    }

    pub async fn station_stats(
        State(app_state): State<AppState>,
        Extension(claims): Extension<Claims>,
    ) -> Result<Json<StationDiscountStatsResponse>, StationError> {
        let station_id = claims.station_res.id;

        let stats = station_discount_stats(&app_state.pool, station_id)
            .await
            .map_err(StationError::DatabaseError)?;

        Ok(Json(StationDiscountStatsResponse {
            redeemed_codes: stats.redeemed_codes,
        }))
    }
}

pub async fn station_discount_stats(
    pool: &PgPool,
    station_id: Uuid,
) -> Result<StationDiscountStats, sqlx::Error> {
    sqlx::query_as::<_, StationDiscountStats>(
        r#"
        SELECT
            COUNT(*) FILTER (WHERE redeemed_at IS NOT NULL)::BIGINT AS redeemed_codes
        FROM discount_codes
        WHERE station_id = $1
        "#,
    )
    .bind(station_id)
    .fetch_one(pool)
    .await
}

pub async fn admin_discount_stats(pool: &PgPool) -> Result<AdminDiscountStats, sqlx::Error> {
    sqlx::query_as::<_, AdminDiscountStats>(
        r#"
        SELECT
            COUNT(*)::BIGINT AS created_codes,
            COUNT(*) FILTER (WHERE redeemed_at IS NOT NULL)::BIGINT AS redeemed_codes
        FROM discount_codes
        "#,
    )
    .fetch_one(pool)
    .await
}
