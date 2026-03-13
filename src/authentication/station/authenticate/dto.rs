use serde::{Deserialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct StationSigninDto {
    pub email: String,
    pub password: String,
    pub station_type: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateStationDto {
    pub name: String,
    pub address: String,
    pub email: String,
    pub phone: String,
    pub password: String,
    pub latitude: f64,
    pub longitude: f64,
    pub code: String,
    pub station_type: String
}

#[derive(Debug, Deserialize)]
pub struct RenewSubscriptionDto {
    pub station_id: Uuid,
    pub days: i64,
    pub super_password: String,
}

