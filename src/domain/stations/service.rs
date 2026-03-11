use crate::{
    app_state::AppState, authentication::station::authenticate::token::service::Claims, 
    domain::{
        stations::model::Station,
        utils::{dto::{AllStationsQuery, StationQueryParam}, errors::station_errors::StationError, schemas::{StationResponse, StationWithCommodity, map_rows_to_stations}, validate_boundary},
    }
};
use axum::{
    Json,
    extract::{Query, Request, State},
};

impl Station {
    pub async fn get_stations(
        State(app_state): State<AppState>,
        Query(query): Query<AllStationsQuery>,
    ) -> Result<Json<Vec<Station>>, StationError> {
        let station_type = query.station_type.unwrap_or("gas".to_string());
        let stations = sqlx::query_as!(
            Station,
            r#"
                SELECT
                    id, name, address, email, phone, 
                    latitude, longitude, role, station_type,
                    created_at, updated_at 
                FROM stations
                WHERE ($1::text IS NULL OR station_type = $1)
            "#,
            station_type
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        Ok(Json(stations))
    }

    pub async fn find_closest_stations(
        State(app_state): State<AppState>,
        Query(query): Query<StationQueryParam>,
    ) -> Result<Json<Vec<StationResponse>>, StationError> {
        let longitude = query.longitude;
        let latitude = query.latitude;
        let station_type = query.station_type.clone();

        let latitude = latitude
            .parse::<f64>()
            .map_err(|_err| StationError::WrongCredentials("latitude".to_string()))?;
        let longitude = longitude
            .parse::<f64>()
            .map_err(|_| StationError::WrongCredentials("longitude".to_string()))?; //todo change it to a better error, may wrong values

        let _ = validate_boundary::validate_abuja_bounds(latitude, longitude)?;

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
                s.station_type AS station_type,
                s.role AS role,
                s.created_at AS created_at,
                s.updated_at AS updated_at,
                haversine($1::float8, $2::float8, s.latitude, s.longitude) AS "distance!",
                c.id AS commodity_id,
                c.name AS commodity_name,
                c.is_available AS "is_available!",
                c.station_id AS "station_id!",
                c.price AS price
            FROM stations AS s
            INNER JOIN commodities AS c ON s.id = c.station_id AND c.is_available = TRUE
            WHERE ($3::text IS NULL OR s.station_type = $3)
              AND s.id IN (
                SELECT sub_s.id
                FROM stations AS sub_s
                WHERE ($3::text IS NULL OR sub_s.station_type = $3)
                  AND EXISTS (
                    SELECT 1 FROM commodities AS sub_c 
                    WHERE sub_c.station_id = sub_s.id AND sub_c.is_available = TRUE
                )
                ORDER BY haversine($1::float8, $2::float8, sub_s.latitude, sub_s.longitude) ASC
                LIMIT 4
            )
            ORDER BY "distance!", s.id, c.name
            "#,
            latitude, 
            longitude,
            station_type
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let station_response = map_rows_to_stations(rows);
    
        Ok(Json(station_response))
    }

    pub async fn get_station(
        State(app_state): State<AppState>,
        request: Request,
    ) -> Result<Json<StationResponse>, StationError> {
        let claims = request
            .extensions()
            .get::<Claims>()
            .ok_or_else(|| StationError::NotFound("Claims not present".to_string()))?;

        let station_id = claims.station_res.id;
        let station_type = claims.station_res.station_type.clone();

        let rows = sqlx::query_as!(
            StationWithCommodity,
            r#"
            SELECT
                s.id, s.name, s.address, s.email, s.password, s.phone,
                s.latitude, s.longitude, s.role, s.created_at, s.station_type,
                s.updated_at, s.distance,
                c.id AS commodity_id,
                c.name AS commodity_name,
                c.price,
                c.is_available,
                c.station_id
            FROM stations s
            LEFT JOIN commodities c ON s.id = c.station_id
            WHERE s.id = $1
              AND s.station_type = $2
            "#,
            station_id,
            station_type
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let station_with_commodities = map_rows_to_stations(rows)
            .into_iter()
            .next()
            .ok_or_else(|| StationError::NotFound("Station not found".to_string()))?;

        Ok(Json(station_with_commodities))
    }
}