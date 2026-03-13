use sqlx::PgPool;
use tokio::time::{Duration, interval};

use super::service::run_subscription_reminder_cycle;

pub async fn start(pool: PgPool) {
    let mut ticker = interval(Duration::from_secs(60 * 60));

    loop {
        ticker.tick().await;
        if let Err(err) = run_subscription_reminder_cycle(&pool).await {
            tracing::error!("subscription reminder cycle failed: {:?}", err);
        }
    }
}
