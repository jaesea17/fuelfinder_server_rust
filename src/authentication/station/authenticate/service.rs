use crate::{
    app_state::AppState,
    authentication::{admin::{dto::CreateRegCodeDto, model::Admins}, station::authenticate::{
        dto::{
             CreateStationDto, RenewSubscriptionDto, StationSigninDto
        },
        token::service::{ApiMessage, TokenService},
    }},
    domain::{
        commodities::model::Commodity,
        registration_code::dto::CodeCreatedMessage,
        stations::model::Station,
        subscriptions::service::{
            create_expired_signin_notification, create_trial_subscription,
            is_station_subscription_expired, renew_subscription_manual,
        },
        utils::{errors::station_errors::StationError, schemas::{CommoditiesResponse, StationResponse, StationWithCommodity, map_rows_to_stations}}
    },
};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use bcrypt;
use bcrypt::BcryptError;
use std::env;

const BCRYPT_COST: u32 = 12;

pub struct Authentication;

impl Authentication {
    pub async fn signup(
        State(app_state): State<AppState>,
        Json(body): Json<CreateStationDto>,
    ) -> Result<impl IntoResponse, StationError> {
        //get name, address,
        // let address = body.address.trim().to_lowercase();
        let code = body.code.trim();
        let email = body.email.trim();
        // check if registration code is still valid
        let is_valid: Option<Option<i32>> = sqlx::query_scalar!(
            "SELECT 1 FROM registration_codes WHERE code = $1 AND is_valid = true",
            code
        )
        .fetch_optional(&app_state.pool)
        .await?;
        if is_valid.flatten().is_none() {
            return Err(StationError::NotFound(
                "invalid registration code".to_string(),
            ));
        };

        let station_type = body.station_type.trim();

        let exists: Option<Option<i32>> = sqlx::query_scalar!("SELECT 1 FROM stations WHERE email = $1 AND station_type = $2", email, station_type)
            .fetch_optional(&app_state.pool)
            .await?;
        if exists.flatten().is_some() {
            return Err(StationError::AlreadyExists);
        };

        let name = body.name.trim().to_uppercase();
        let address = body.address.trim();
        let phone = body.phone.trim();
        let password = body.password;
        let latitude = body.latitude;
        let longitude = body.longitude;
        //hash the password
        let hashed_password = Authentication::hash_password(&password)
            .await
            .expect("Couldn't hash password"); // TODO handle gracefully bcrypt err

        //Create either a petrol_station or gas_station   
        let new_station: Station = sqlx::query_as!(
                Station,
                r#"
                    INSERT INTO stations (
                        name, address, email, phone, password, latitude, longitude, station_type
                    )
                    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                    RETURNING id, name, address, email, phone, latitude, longitude, role, station_type, created_at, updated_at
                "#,
                name, address, email, phone, hashed_password, latitude, longitude, station_type
            )
            .fetch_one(&app_state.pool)
            .await
            .map_err(StationError::DatabaseError)?;
        let station_id = new_station.id;

        create_trial_subscription(&app_state.pool, station_id)
            .await
            .map_err(|err| StationError::WrongCredentials(err.to_string()))?;

        let name = station_type;

        //Creating Petrol or Gas commodity
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
        .map_err(StationError::DatabaseError)?;

        // Update the registration_codes table
        sqlx::query!(
            r#"
                UPDATE registration_codes
                SET
                    station_id = $1,
                    is_valid = $2
                WHERE code = $3
            "#,
            station_id,
            false,
            code
        )
        .execute(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let mut new_station: StationResponse = new_station.into();
        new_station.commodities = vec![CommoditiesResponse {
            id: new_commodity.id,
            name: new_commodity.name,
            is_available: new_commodity.is_available,
            price: new_commodity.price,
            station_id: new_commodity.station_id,
            discount_enabled: None,
            discount_percentage: None,
        }];
        Ok((StatusCode::CREATED, Json(new_station)))
    }

//------------------------------------------------------------------------------

    pub async fn signin(
        State(app_state): State<AppState>,
        Json(body): Json<StationSigninDto>,
    ) -> Result<impl IntoResponse, StationError> {
        let StationSigninDto { email, password, station_type } = body;

        // Use the struct that contains both Station and Commodity fields
        let rows: Vec<_> = sqlx::query_as!(
            StationWithCommodity,
            r#"
                SELECT 
                    s.id, s.name, s.address, s.email, s.phone, 
                    s.latitude, s.longitude, s.role, s.station_type,
                    s.password, s.created_at, s.updated_at,
                    NULL::float8 as "distance?", 
                    c.id as commodity_id, 
                    c.name as commodity_name, 
                    c.is_available as "is_available!", 
                    c.price as price,
                    c.station_id as "station_id!",
                    NULL::bool as "discount_enabled?",
                    NULL::int4 as "discount_percentage?"
                FROM stations s
                INNER JOIN commodities c ON s.id = c.station_id
                WHERE s.email = $1 AND s.station_type = $2
            "#,
            email, station_type
        )
        .fetch_all(&app_state.pool) // Use fetch_all because one station has many commodities
        .await
        .map_err(StationError::DatabaseError)?;

        // Check if we actually found the station
        if rows.is_empty() {
            return Err(StationError::NotFound(email));
        }

        let stored_hash = &rows[0].password;

        match Authentication::verify_password(&password, stored_hash).await {
            Ok(false) | Err(_) => {
                // ❌ Password is INCORRECT or verification failed: Authentication fails
                Err(StationError::WrongCredentials(
                    "email or password".to_string(),
                ))
            }

            Ok(true) => {
                let station_id = rows[0].id;
                let is_expired = is_station_subscription_expired(&app_state.pool, station_id)
                    .await
                    .map_err(|err| StationError::WrongCredentials(err.to_string()))?;

                if is_expired {
                    create_expired_signin_notification(&app_state.pool, station_id)
                        .await
                        .map_err(|err| StationError::WrongCredentials(err.to_string()))?;

                    return Err(StationError::WrongCredentials(
                        "subscription expired".to_string(),
                    ));
                }

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

    pub async fn renew_subscription(
        State(app_state): State<AppState>,
        Json(body): Json<RenewSubscriptionDto>,
    ) -> Result<impl IntoResponse, StationError> {
        let RenewSubscriptionDto {
            station_id,
            days,
            super_password,
        } = body;

        let admin: Admins = sqlx::query_as!(
            Admins,
            r#"
            SELECT
              id, role, password, created_at, updated_at
            FROM admins
            LIMIT 1
            "#
        )
        .fetch_one(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let is_verified = Authentication::verify_password(&super_password, &admin.password)
            .await
            .map_err(|_| StationError::WrongCredentials("admin password".to_string()))?;

        if !is_verified {
            return Err(StationError::WrongCredentials("admin password".to_string()));
        }

        renew_subscription_manual(&app_state.pool, station_id, admin.id, days)
            .await
            .map_err(|err| StationError::WrongCredentials(err.to_string()))?;

        Ok((
            StatusCode::OK,
            Json(CodeCreatedMessage {
                code: "subscription renewed".to_string(),
            }),
        ))
    }

    pub async fn create_reg_code(
        State(app_state): State<AppState>,
        Json(body): Json<CreateRegCodeDto>
    ) -> Result<impl IntoResponse, StationError> {
        //Confirm the Super adim password is correct
        let CreateRegCodeDto { code, super_password } = body;
        let admin: Admins = sqlx::query_as!(
            Admins,
            r#"
            SELECT
              id, role, password, created_at, updated_at
            FROM Admins 
            "#
        )
        .fetch_one(&app_state.pool)
        .await
        .map_err(StationError::DatabaseError)?;

        let stored_hashed = admin.password;

        let is_verified = Authentication::verify_password(&super_password, &stored_hashed)
            .await
            .map_err(|_| StationError::WrongCredentials("admin password".to_string()))?;

        if !is_verified {
            return Err(StationError::WrongCredentials("admin password".to_string()));
        }

        sqlx::query!(
                r#"
                    INSERT INTO registration_codes (
                        code
                    )
                    VALUES ($1)
                "#,
                code
            )
            .execute(&app_state.pool)
            .await
            .map_err(StationError::DatabaseError)?;

         Ok((
                StatusCode::OK,
                Json( CodeCreatedMessage {
                    code,
                }),
            )
                .into_response())

    }

//---------------------------------------------------------------------

    pub async fn hash_password(plaintext_password: &str) -> Result<String, bcrypt::BcryptError> {
        let hashed_password = bcrypt::hash(plaintext_password, BCRYPT_COST)?;

        Ok(hashed_password)
    }

//-----------------------------------------------------------------------

    pub async fn verify_password(
        plaintext_password: &str,
        stored_hash: &str,
    ) -> Result<bool, BcryptError> {
        let is_valid = bcrypt::verify(plaintext_password, stored_hash)?;

        Ok(is_valid)
    }
}
