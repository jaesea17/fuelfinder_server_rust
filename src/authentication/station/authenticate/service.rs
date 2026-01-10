use crate::{
    app_state::AppState,
    authentication::station::authenticate::{
        dto::{
            CommoditiesResponse, CreateStationDto, StationResponse, StationSigninDto,
            StationWithCommodity, map_rows_to_stations,
        },
        token::service::{ApiMessage, TokenService},
    },
    domain::{
        commodities::commodity::Commodity, stations::station::Station,
        utils::errors::station_errors::StationError,
    },
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use bcrypt;
use bcrypt::BcryptError;
use std::env;

const BCRYPT_COST: u32 = 12;

impl StationWithCommodity {
    pub async fn signin(
        State(app_state): State<AppState>,
        Json(body): Json<StationSigninDto>,
    ) -> Result<impl IntoResponse, StationError> {
        let StationSigninDto { email, password } = body;

        // Use the struct that contains both Station and Commodity fields
        let rows: Vec<_> = sqlx::query_as!(
            StationWithCommodity,
            r#"
                SELECT 
                    s.id, s.name, s.address, s.email, s.phone, 
                    s.latitude, s.longitude, s.role, s.password,
                    s.created_at, s.updated_at,
                    -- Add distance calculation here if you need it, else keep it as NULL/Option
                    NULL::float8 as "distance?", 
                    c.id as commodity_id, 
                    c.name as commodity_name, 
                    c.is_available as "is_available!", 
                    c.price as price,
                    c.station_id as "station_id!"
                FROM stations s
                INNER JOIN commodities c ON s.id = c.station_id
                WHERE s.email = $1
            "#,
            email
        )
        .fetch_all(&app_state.pool) // Use fetch_all because one station has many commodities
        .await
        .map_err(|err| StationError::DatabaseError(err))?;

        // Check if we actually found the station
        if rows.is_empty() {
            return Err(StationError::NotFound(email));
        }

        let stored_hash = &rows[0].password;

        match StationWithCommodity::verify_password(&password, stored_hash).await {
            Ok(false) | Err(_) => {
                // ❌ Password is INCORRECT or verification failed: Authentication fails
                Err(StationError::WrongCredentials(
                    "email or password".to_string(),
                ))
            }

            Ok(true) => {
                // ✅ Password is CORRECT: Proceed with successful authentication
                // Create JWT token...
                let jwt_secret = env::var("JWT_SECRET")
                    .expect("JWT_SECRET must be set in the environment or .env file");
                let token_service = TokenService::new(&jwt_secret);
                //Check out station/signin/dto to see the implementation of
                //From trait that aids line 72.
                // To convert the vector of stations into a
                //more organised result of Station that has a column of Vec<Commodities>
                let station_response = map_rows_to_stations(rows).swap_remove(0);
                // let station_response = map_rows_to_stations(rows)
                //     .iter()
                //     .next()
                //     .ok_or(|| StationError::NotFound("No station found".to_string()));

                let jwt_token = token_service
                    .create_token(station_response)
                    .map_err(|err| StationError::WrongCredentials(err.to_string()))?;
                let jwt_token = jwt_token.to_string();

                //TODO update the Station's is_logged_in status to true

                Ok((
                    StatusCode::OK,
                    Json(ApiMessage {
                        access_token: jwt_token,
                    }),
                )
                    .into_response())
            }
        }
    }

    pub async fn signup(
        State(app_state): State<AppState>,
        Json(body): Json<CreateStationDto>,
    ) -> Result<impl IntoResponse, StationError> {
        //get name, address,
        // let address = body.address.trim().to_lowercase();
        let email = body.email.trim();
        let exists: Option<Option<i32>> =
            sqlx::query_scalar!("SELECT 1 FROM stations WHERE email = $1", email)
                .fetch_optional(&app_state.pool)
                .await?;

        if exists.is_some_and(|x| x.is_some() == true) {
            return Err(StationError::AlreadyExists);
        };

        let CreateStationDto {
            name,
            address,
            email,
            phone,
            password,
            latitude,
            longitude,
        } = body;

        //hash the password
        let hashed_password = StationWithCommodity::hash_password(&password)
            .await
            .expect("Couldn't hash password"); // TODO handle gracefully bcrypt err

        let new_station = sqlx::query_as!(
            Station,
            r#"
                INSERT INTO stations (
                    name, address, email, phone, password, latitude, longitude
                )
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                RETURNING id, name, address, email, phone, latitude, longitude, role, created_at, updated_at
            "#,
            name, address, email, phone, hashed_password, latitude, longitude
        )
        .fetch_one(&app_state.pool)
        .await
        .map_err(|err| StationError::DatabaseError(err))?;

        let station_id = new_station.id;
        let name = "petrol";
        //Creating Petrol commodity
        let new_commodity: Commodity = sqlx::query_as!(
            Commodity,
            r#"
                INSERT INTO commodities (
                    name, station_id
                )
                VALUES ($1, $2)
                RETURNING name, id, price, is_available, created_at, updated_at, station_id
            "#,
            name,
            station_id
        )
        .fetch_one(&app_state.pool)
        .await
        .map_err(|err| StationError::DatabaseError(err))?;

        let mut new_station: StationResponse = new_station.into();
        new_station.commodities = vec![CommoditiesResponse {
            id: new_commodity.id,
            name: new_commodity.name,
            is_available: new_commodity.is_available,
            price: new_commodity.price,
            station_id: new_commodity.station_id,
        }];
        Ok((StatusCode::CREATED, Json(new_station)))
    }

    pub async fn hash_password(plaintext_password: &str) -> Result<String, bcrypt::BcryptError> {
        let hashed_password = bcrypt::hash(plaintext_password, BCRYPT_COST)?;

        Ok(hashed_password)
    }

    pub async fn verify_password(
        plaintext_password: &str,
        stored_hash: &str,
    ) -> Result<bool, BcryptError> {
        let is_valid = bcrypt::verify(plaintext_password, stored_hash)?;

        Ok(is_valid)
    }
}
