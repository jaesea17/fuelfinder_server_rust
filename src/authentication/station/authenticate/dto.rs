use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::stations::station::Station;
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
pub struct StationSigninDto {
    pub email: String,
    pub password: String,
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
}

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct StationWithCommodity {
    // Columns from the 'stations' table
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub email: String,
    pub password: String,
    pub phone: String,
    pub latitude: f64,
    pub longitude: f64,
    pub role: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
    pub distance: Option<f64>,

    // Columns from the 'commodities' table
    pub commodity_id: Uuid,
    pub commodity_name: String,
    pub is_available: bool,
    pub price: i32,
    pub station_id: Uuid,
}

#[derive(Serialize, Debug, Clone, Deserialize, sqlx::FromRow)]
pub struct StationResponse {
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
    pub distance: Option<f64>,

    pub commodities: Vec<CommoditiesResponse>,
}

#[derive(Serialize, Debug, Clone, Deserialize, sqlx::FromRow)]
pub struct CommoditiesResponse {
    pub id: Uuid,
    pub name: String,
    pub is_available: bool,
    pub price: i32,
    pub station_id: Uuid,
}

impl From<Vec<StationWithCommodity>> for StationResponse {
    fn from(rows: Vec<StationWithCommodity>) -> Self {
        // We assume the caller checked that rows is not empty.
        // If it might be empty, use TryFrom instead.
        let first = &rows[0];

        Self {
            id: first.id,
            name: first.name.clone(),
            address: first.address.clone(),
            email: first.email.clone(),
            phone: first.phone.clone(),
            latitude: first.latitude,
            longitude: first.longitude,
            role: first.role.clone(),
            created_at: first.created_at,
            updated_at: first.updated_at,
            distance: first.distance, // Carrying over the Option<f64>

            // Map each row's commodity fields into the nested struct
            commodities: rows
                .into_iter()
                .map(|row| CommoditiesResponse {
                    id: row.commodity_id,
                    name: row.commodity_name,
                    is_available: row.is_available,
                    price: row.price,
                    station_id: row.station_id,
                })
                .collect(),
        }
    }
}

pub fn map_rows_to_stations(rows: Vec<StationWithCommodity>) -> Vec<StationResponse> {
    if rows.len() == 0 {
        println!("*** NO STATIONS FOUND!! ***");
    };
    let mut station_map: HashMap<Uuid, StationResponse> = HashMap::new();

    for row in rows {
        // Entry API: find the station or create it if it doesn't exist
        let station = station_map
            .entry(row.id)
            .or_insert_with(|| StationResponse {
                id: row.id,
                name: row.name.clone(),
                address: row.address.clone(),
                email: row.email.clone(),
                phone: row.phone.clone(),
                latitude: row.latitude,
                longitude: row.longitude,
                role: row.role.clone(),
                created_at: row.created_at,
                updated_at: row.updated_at,
                distance: row.distance,
                commodities: Vec::new(),
            });

        // Add the specific commodity from this row to the station's list
        station.commodities.push(CommoditiesResponse {
            id: row.commodity_id,
            name: row.commodity_name.clone(),
            is_available: row.is_available,
            price: row.price,
            station_id: row.station_id,
        });
    }

    // Convert the HashMap into a Vec
    let mut result: Vec<StationResponse> = station_map.into_values().collect();

    if result.len() == 1 {
        return result;
    }
    // Re-sort by distance since HashMaps are unordered
    result.sort_by(|a, b| {
        a.distance
            .partial_cmp(&b.distance)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    result
}

impl From<Station> for StationResponse {
    fn from(station: Station) -> Self {
        Self {
            id: station.id,
            name: station.name.clone(),
            address: station.address.clone(),
            email: station.email.clone(),
            phone: station.phone.clone(),
            latitude: station.latitude,
            longitude: station.longitude,
            role: station.role.clone(),
            created_at: station.created_at,
            updated_at: station.updated_at,
            distance: Some(0.0),
            commodities: vec![],
        }
    }
}
