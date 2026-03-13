use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRegCodeDto {
    pub code: String,
    pub super_password: String,
}

#[derive(Debug, Deserialize)]
pub struct AdminStationsQuery {
    pub filter: Option<String>,
}