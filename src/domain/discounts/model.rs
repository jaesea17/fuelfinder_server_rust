use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DiscountCode {
    pub id: Uuid,
    pub code: String,
    pub station_id: Uuid,
    pub commodity_id: Uuid,
    pub created_price: i32,
    pub discount_percentage: i32,
    pub discounted_price: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub redeemed_at: Option<DateTime<Utc>>,
    pub redeemed_by_station_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct StationDiscountStats {
    pub redeemed_codes: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct AdminDiscountStats {
    pub created_codes: i64,
    pub redeemed_codes: i64,
}
