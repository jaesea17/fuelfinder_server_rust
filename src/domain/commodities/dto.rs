use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct UpdateCommodityDto {
    pub price: i32,
    pub is_available: Option<bool>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct UpdateCommodityResponse {
    pub id: Uuid,
    pub price: i32,
    pub is_available: bool,
    pub updated_at: chrono::NaiveDateTime,
}
