use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct RegistrationCode {
    pub id: Uuid,
    pub station_id: Uuid,
    pub code: String,
    pub is_valid: bool,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Serialize)]
pub struct CodeCreatedMessage{
    pub code: String
}