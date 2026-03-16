use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRegCodeDto {
    pub code: String,
    pub super_password: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminStationsQuery {
    pub filter: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCommodityDiscountDto {
    pub commodity_id: Uuid,
    pub enabled: bool,
    pub percentage: Option<i32>,
}