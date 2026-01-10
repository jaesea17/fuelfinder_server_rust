use crate::{
    app_state::AppState,
    domain::{
        commodities::dto::{UpdateCommodityDto, UpdateCommodityResponse},
        utils::errors::commodity_errors::CommodityError,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono;
use http::StatusCode;
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

impl Commodity {
    pub async fn get_commodities(
        State(app_state): State<AppState>,
    ) -> Result<Json<Vec<Commodity>>, CommodityError> {
        let commodities = sqlx::query_as!(
            Commodity,
            r#"
                SELECT
                    id, name, price, is_available, station_id,
                    created_at, updated_at 
                FROM commodities
            "#
        )
        .fetch_all(&app_state.pool)
        .await
        .map_err(|err| CommodityError::DatabaseError(err))?; // Use ? for error propagation

        Ok(Json(commodities))
    }

    pub async fn update_commodity(
        State(app_state): State<AppState>,
        Path(id): Path<Uuid>,
        // Extension(current_user): Extension<Claims>, // Directly get the auth data
        Json(payload): Json<UpdateCommodityDto>,
    ) -> Result<impl IntoResponse, CommodityError> {
        let price = payload.price;
        let is_available = payload.is_available.unwrap_or_default();

        // let price = price.parse::<i32>().map_err(|_| {
        //     CommodityError::WrongCredentials("couldn't parse price format".to_string())
        // })?;

        let updated_commodity = sqlx::query_as!(
            UpdateCommodityResponse,
            r#"
                UPDATE commodities
                SET 
                    price = $1,
                    is_available = $2,
                    updated_at = NOW()
                WHERE id = $3
                RETURNING 
                    id, 
                    price AS "price!", 
                    is_available AS "is_available!", 
                    updated_at AS "updated_at!"
            "#,
            price,
            is_available,
            id
        )
        .fetch_optional(&app_state.pool)
        .await
        .map_err(|e| CommodityError::DatabaseError(e))?
        // Use ok_or to convert Option to Result
        .ok_or_else(|| CommodityError::NotFound(id.to_string()))?;

        Ok((StatusCode::OK, Json(updated_commodity)))
    }
}
