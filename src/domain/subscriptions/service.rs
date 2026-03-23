use anyhow::Context;
use chrono::{Duration, Utc};
use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
    transport::smtp::authentication::Credentials,
};
use sqlx::PgPool;
use std::env;
use uuid::Uuid;

use super::model::{DashboardNotification, ReminderType, Subscription};

const SUBSCRIPTION_KIND: &str = "subscription";

pub async fn create_trial_subscription(pool: &PgPool, station_id: Uuid) -> anyhow::Result<()> {
    let starts_at = Utc::now();
    let ends_at = starts_at + Duration::days(30);

    sqlx::query(
        r#"
        INSERT INTO subscriptions (station_id, starts_at, ends_at, status)
        VALUES ($1, $2, $3, 'active')
        "#,
    )
    .bind(station_id)
    .bind(starts_at)
    .bind(ends_at)
    .execute(pool)
    .await
    .context("failed to create signup trial subscription")?;

    Ok(())
}

pub async fn renew_subscription_manual(
    pool: &PgPool,
    station_id: Uuid,
    admin_id: Uuid,
    days: i64,
) -> anyhow::Result<()> {
    let starts_at = Utc::now();
    let ends_at = starts_at + Duration::days(days.max(1));

    let mut tx = pool.begin().await?;

    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'expired'
        WHERE station_id = $1 AND status = 'active'
        "#,
    )
    .bind(station_id)
    .execute(&mut *tx)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO subscriptions (station_id, starts_at, ends_at, status, created_by_admin)
        VALUES ($1, $2, $3, 'active', $4)
        "#,
    )
    .bind(station_id)
    .bind(starts_at)
    .bind(ends_at)
    .bind(admin_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(())
}

pub async fn is_station_subscription_expired(pool: &PgPool, station_id: Uuid) -> anyhow::Result<bool> {
    let latest = sqlx::query_as::<_, Subscription>(
        r#"
        SELECT id, station_id, starts_at, ends_at, status, created_at
        FROM subscriptions
        WHERE station_id = $1
        ORDER BY created_at DESC
        LIMIT 1
        "#,
    )
    .bind(station_id)
    .fetch_optional(pool)
    .await?;

    let now = Utc::now();

    match latest {
        None => Ok(true),
        Some(sub) => Ok(sub.status != "active" || now >= sub.ends_at),
    }
}

pub async fn create_dashboard_notification(
    pool: &PgPool,
    station_id: Uuid,
    title: &str,
    body: &str,
    kind: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        INSERT INTO notifications (station_id, title, body, kind)
        VALUES ($1, $2, $3, $4)
        "#,
    )
    .bind(station_id)
    .bind(title)
    .bind(body)
    .bind(kind)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn create_expired_signin_notification(pool: &PgPool, station_id: Uuid) -> anyhow::Result<()> {
    let body = "Your subscription has expired. Please contact admin for renewal.";

    create_dashboard_notification(
        pool,
        station_id,
        "Subscription expired",
        body,
        SUBSCRIPTION_KIND,
    )
    .await
}

pub async fn get_station_notifications(
    pool: &PgPool,
    station_id: Uuid,
) -> anyhow::Result<Vec<DashboardNotification>> {
    let notifications = sqlx::query_as::<_, DashboardNotification>(
        r#"
        SELECT id, title, body, kind, is_read, created_at
        FROM notifications
        WHERE station_id = $1
        ORDER BY created_at DESC
        LIMIT 50
        "#,
    )
    .bind(station_id)
    .fetch_all(pool)
    .await?;

    Ok(notifications)
}

pub async fn mark_station_notification_read(
    pool: &PgPool,
    station_id: Uuid,
    notification_id: Uuid,
) -> anyhow::Result<bool> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE notifications
        SET is_read = TRUE
        WHERE id = $1 AND station_id = $2
        "#,
    )
    .bind(notification_id)
    .bind(station_id)
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected > 0)
}

async fn create_reminder_log_once(
    pool: &PgPool,
    subscription_id: Uuid,
    reminder_type: ReminderType,
) -> anyhow::Result<bool> {
    let rows_affected = sqlx::query(
        r#"
        INSERT INTO subscription_reminder_logs (subscription_id, reminder_type)
        VALUES ($1, $2)
        ON CONFLICT (subscription_id, reminder_type) DO NOTHING
        "#,
    )
    .bind(subscription_id)
    .bind(reminder_type.as_str())
    .execute(pool)
    .await?
    .rows_affected();

    Ok(rows_affected > 0)
}

async fn mark_subscription_expired(pool: &PgPool, subscription_id: Uuid) -> anyhow::Result<()> {
    sqlx::query(
        r#"
        UPDATE subscriptions
        SET status = 'expired'
        WHERE id = $1 AND status = 'active'
        "#,
    )
    .bind(subscription_id)
    .execute(pool)
    .await?;

    Ok(())
}

async fn send_subscription_email(
    _pool: &PgPool,
    station_email: &str,
    subject: &str,
    body: &str,
) -> anyhow::Result<()> {
    let smtp_host = match env::var("SMTP_HOST") {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!("SMTP_HOST missing; skipping email send to {station_email}");
            return Ok(());
        }
    };

    let smtp_port = env::var("SMTP_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(587);
    let smtp_username = env::var("SMTP_USERNAME").unwrap_or_default();
    let smtp_password = env::var("SMTP_PASSWORD").unwrap_or_default();
    let smtp_from = env::var("SMTP_FROM").unwrap_or_else(|_| "noreply@fuelgetter.local".to_string());

    let email = Message::builder()
        .from(smtp_from.parse()?)
        .to(station_email.parse()?)
        .subject(subject)
        .body(body.to_string())?;

    let creds = Credentials::new(smtp_username, smtp_password);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&smtp_host)?
        .credentials(creds)
        .port(smtp_port)
        .build();

    if let Err(err) = mailer.send(email).await {
        tracing::error!("failed to send subscription email to {}: {:?}", station_email, err);
    }

    Ok(())
}

fn eligible_reminder_type(time_left: Duration) -> Option<ReminderType> {
    if time_left <= Duration::zero() {
        return None;
    }

    let whole_days_left = time_left.num_days();

    match whole_days_left {
        7 => Some(ReminderType::D7),
        4 => Some(ReminderType::D4),
        0 | 1 => Some(ReminderType::D1),
        _ => None,
    }
}

pub async fn run_subscription_reminder_cycle(pool: &PgPool) -> anyhow::Result<()> {
    let subscriptions = sqlx::query_as::<_, Subscription>(
        r#"
        SELECT id, station_id, starts_at, ends_at, status, created_at
        FROM subscriptions
        WHERE status = 'active'
        "#,
    )
    .fetch_all(pool)
    .await?;

    let now = Utc::now();

    for subscription in subscriptions {
        let station_email: Option<String> = sqlx::query_scalar(
            r#"SELECT email FROM stations WHERE id = $1"#,
        )
        .bind(subscription.station_id)
        .fetch_optional(pool)
        .await?;

        let Some(station_email) = station_email else {
            continue;
        };

        if now >= subscription.ends_at {
            if create_reminder_log_once(pool, subscription.id, ReminderType::Expired).await? {
                let body = "Your subscription has expired. Please contact admin for renewal.";
                create_dashboard_notification(
                    pool,
                    subscription.station_id,
                    "Subscription expired",
                    body,
                    SUBSCRIPTION_KIND,
                )
                .await?;
                send_subscription_email(pool, &station_email, "Subscription expired", body).await?;
            }

            mark_subscription_expired(pool, subscription.id).await?;
            continue;
        }

        let time_left = subscription.ends_at - now;
        let Some(reminder_type) = eligible_reminder_type(time_left) else {
            continue;
        };

        if !create_reminder_log_once(pool, subscription.id, reminder_type).await? {
            continue;
        }

        let day_count = reminder_type.days_left();
        let body = format!(
            "Your subscription expires in {} day(s). Please renew to avoid interruption.",
            day_count
        );

        create_dashboard_notification(
            pool,
            subscription.station_id,
            "Subscription reminder",
            &body,
            SUBSCRIPTION_KIND,
        )
        .await?;

        let subject = format!("Subscription expires in {} day(s)", day_count);
        send_subscription_email(pool, &station_email, &subject, &body).await?;
    }

    Ok(())
}
