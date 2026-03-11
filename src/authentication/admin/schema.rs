use uuid::Uuid;

pub struct ReturnedAdmin {
    pub id: Uuid,
    pub role: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}