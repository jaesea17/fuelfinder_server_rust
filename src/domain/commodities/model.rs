use chrono;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct Commodity {
    pub id: Uuid,
    pub name: String,
    pub price: i32,
    pub station_id: Uuid,
    pub is_available: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
