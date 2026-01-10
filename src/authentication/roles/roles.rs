use serde::{Deserialize, Serialize};

#[derive(PartialEq, Serialize, Deserialize, sqlx::Type)]
pub struct UserRole {
    pub admin: String,
    pub station: String,
    pub user: String,
}

impl UserRole {
    pub fn new() -> Self {
        UserRole {
            admin: "admin".to_string(),
            station: "station".to_string(),
            user: "user".to_string(),
        }
    }
}
