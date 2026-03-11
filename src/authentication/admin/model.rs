use uuid::Uuid;

pub struct Admins{
    pub id: Uuid,
    pub role: String,
    pub password: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}