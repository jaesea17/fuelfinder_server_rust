use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;
use chrono;


#[derive(Debug, Serialize, FromRow, Deserialize)]
pub struct Station {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub latitude: f64,
    pub longitude: f64,
    pub role: String,
    pub station_type: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
