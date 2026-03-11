use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize)]
pub struct CreateRegCodeDto{
    pub code: String,
    pub super_password: String
}