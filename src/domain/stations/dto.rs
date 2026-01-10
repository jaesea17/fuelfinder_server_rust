use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct StationQueryParam {
    pub longitude: String,
    pub latitude: String,
}
#[derive(Debug, sqlx::FromRow)]
pub struct Station {
    // Columns from the 'stations' table
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub latitude: f64,
    pub longitude: f64,
    pub role: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub distance_km: Option<f64>,
}
