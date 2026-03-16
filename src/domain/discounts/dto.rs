use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct GenerateDiscountCodeDto {
    pub station_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct DiscountCodeResponse {
    pub code: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub discount_percentage: i32,
    pub original_price: i32,
    pub discounted_price: i32,
    pub is_expired: bool,
}

#[derive(Debug, Deserialize)]
pub struct RedeemDiscountCodeDto {
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct RedeemDiscountCodeResponse {
    pub message: String,
    pub code: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub discount_percentage: Option<i32>,
    pub discounted_price: Option<i32>,
    pub is_expired: bool,
}

#[derive(Debug, Serialize)]
pub struct StationDiscountStatsResponse {
    pub redeemed_codes: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminDiscountStatsResponse {
    pub created_codes: i64,
    pub redeemed_codes: i64,
}
