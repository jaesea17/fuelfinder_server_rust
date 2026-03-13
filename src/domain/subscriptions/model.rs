use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct Subscription {
    pub id: Uuid,
    pub station_id: Uuid,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub status: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct DashboardNotification {
    pub id: Uuid,
    pub title: String,
    pub body: String,
    pub kind: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy)]
pub enum ReminderType {
    D7,
    D4,
    D1,
    Expired,
}

impl ReminderType {
    pub fn as_str(self) -> &'static str {
        match self {
            ReminderType::D7 => "d7",
            ReminderType::D4 => "d4",
            ReminderType::D1 => "d1",
            ReminderType::Expired => "expired",
        }
    }

    pub fn days_left(self) -> i64 {
        match self {
            ReminderType::D7 => 7,
            ReminderType::D4 => 4,
            ReminderType::D1 => 1,
            ReminderType::Expired => 0,
        }
    }
}
