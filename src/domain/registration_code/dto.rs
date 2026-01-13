use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RegistrationCode {
    pub id: Uuid,
    pub station_id: Uuid,
    pub code: String,
    pub is_valid: bool,
    pub created_at: chrono::NaiveDateTime,
}
