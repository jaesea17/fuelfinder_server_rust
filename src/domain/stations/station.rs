use crate::{
    app_state::AppState, authentication::{ roles::roles::UserRole, station::authenticate::{dto::{CommoditiesResponse, StationResponse, StationWithCommodity, map_rows_to_stations}, token::service::Claims}}, domain::{
        stations::dto::StationQueryParam,
        utils::{errors::station_errors::StationError, validate_boundary},
    }
};
use axum::{
    Json,
    extract::{Query, Request, State},
};
use chrono;
use http::request;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

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
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl Station {
    pub async fn get_stations(
        State(app_state): State<AppState>,
    ) -> Result<Json<Vec<Station>>, StationError> {
        let stations = sqlx::query_as!(
            Station,
            r#"
                SELECT
                    id, name, address, email, phone, 
                    latitude, longitude, role,
                    created_at, updated_at 
                FROM stations
            "#
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(|err| StationError::DatabaseError(err))?; // Use ? for error propagation

        Ok(Json(stations))
    }

    pub async fn find_closest_stations(
        State(app_state): State<AppState>,
        Query(query): Query<StationQueryParam>,
    ) -> Result<Json<Vec<StationResponse>>, StationError> {
        let longitude = query.longitude;
        let latitude = query.latitude;

        let latitude = latitude
            .parse::<f64>()
            .map_err(|_err| StationError::WrongCredentials("latitude".to_string()))?; //todo change it to a better error, may wrong values
        let longitude = longitude
            .parse::<f64>()
            .map_err(|_| StationError::WrongCredentials("longitude".to_string()))?; //todo change it to a better error, may wrong values

        let _ = validate_boundary::validate_abuja_bounds(latitude, longitude);

        let rows = sqlx::query_as!(
            StationWithCommodity,
            r#"
            SELECT
                s.id AS id,
                s.name AS name,
                s.address AS address,
                s.email AS email,
                s.password AS password,
                s.phone AS phone,
                s.latitude AS latitude,
                s.longitude AS longitude,
                s.role AS role,
                s.created_at AS created_at,
                s.updated_at AS updated_at,
                -- Call your custom function here to populate the column
                haversine($1::float8, $2::float8, s.latitude, s.longitude) AS "distance!",

                c.id AS commodity_id,
                c.name AS commodity_name,
                c.is_available AS "is_available!",
                c.station_id AS "station_id!",
                c.price AS price
            FROM stations AS s
            
            INNER JOIN commodities AS c ON s.id = c.station_id AND c.is_available = TRUE
            
            WHERE s.id IN (
                SELECT sub_s.id
                FROM stations AS sub_s
                WHERE EXISTS (
                    SELECT 1 FROM commodities AS sub_c 
                    WHERE sub_c.station_id = sub_s.id AND sub_c.is_available = TRUE
                )
                -- Keep the ordering here to pick the correct 4 IDs
                ORDER BY haversine($1::float8, $2::float8, sub_s.latitude, sub_s.longitude) ASC
                LIMIT 4
            )
            -- Grouping by ID and then sorting by distance so the nearest station is first in your Rust Vec
            ORDER BY "distance!", s.id, c.name
            "#,
            latitude, 
            longitude
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(|err| StationError::DatabaseError(err))?;

        let station_response = map_rows_to_stations(rows);
    
        Ok(Json(station_response))
    }

    pub async fn get_station(
        State(app_state): State<AppState>,
        request: Request,
    ) -> Result<Json<StationResponse>, StationError> {
        let claims = request.extensions().get::<Claims>().ok_or_else(|| StationError::NotFound("Claims not present".to_string()))?;
        let station_id = claims.station_res.id.clone();

       let rows = sqlx::query_as!(
            StationWithCommodity,
            r#"
            SELECT
                s.id, s.name, s.address, s.email, s.password, s.phone,
                s.latitude, s.longitude, s.role, s.created_at,
                s.updated_at, s.distance,
                c.id AS commodity_id,
                c.name AS commodity_name,
                c.price,
                c.is_available,
                c.station_id
            FROM stations s
            LEFT JOIN commodities c ON s.id = c.station_id
            WHERE s.id = $1
            "#,
            station_id
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let station_with_commodities = map_rows_to_stations(rows)[0].clone();

        Ok(Json(station_with_commodities))
    }
}
