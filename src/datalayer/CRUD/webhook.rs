use crate::errors::errors::ServiceError;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Webhook {
    pub id: Uuid,
    pub account_id: Uuid,
    pub url: String,
    pub secret: String,
    pub events: serde_json::Value, // JSONB array
    pub status: String,
    pub max_retries: Option<i32>,
    pub retry_backoff_seconds: Option<i32>,
    pub consecutive_failures: Option<i32>,
    pub last_failure_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new webhook for an account
pub async fn create_webhook(
    account_id: Uuid,
    url: String,
    secret: String,
    conn: &mut PgConnection,
) -> Result<Webhook, ServiceError> {
    let webhook = sqlx::query_as::<_, Webhook>(
        r#"
        INSERT INTO webhooks (account_id, url, secret, status)
        VALUES ($1, $2, $3, 'active')
        RETURNING *
        "#,
    )
    .bind(account_id)
    .bind(&url)
    .bind(&secret)
    .fetch_one(conn)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "Failed to create webhook");
        ServiceError::DatabaseError(e.to_string())
    })?;

    tracing::info!(
        webhook_id = %webhook.id,
        account_id = %account_id,
        url = %url,
        "Webhook created successfully"
    );

    Ok(webhook)
}

/// Get active webhooks for an account
pub async fn get_active_webhooks_for_account(
    account_id: Uuid,
    conn: &mut PgConnection,
) -> Result<Vec<Webhook>, sqlx::Error> {
    sqlx::query_as::<_, Webhook>(
        "SELECT * FROM webhooks WHERE account_id = $1 AND status = 'active'",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await
}

/// Get all webhooks for an account (including inactive)
pub async fn get_webhooks_for_account(
    account_id: Uuid,
    conn: &mut PgConnection,
) -> Result<Vec<Webhook>, ServiceError> {
    let webhooks = sqlx::query_as::<_, Webhook>(
        "SELECT * FROM webhooks WHERE account_id = $1 ORDER BY created_at DESC",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, account_id = %account_id, "Failed to fetch webhooks");
        ServiceError::DatabaseError(e.to_string())
    })?;

    Ok(webhooks)
}

/// Get a specific webhook by ID
pub async fn get_webhook_by_id(
    webhook_id: Uuid,
    account_id: Uuid,
    conn: &mut PgConnection,
) -> Result<Webhook, ServiceError> {
    let webhook =
        sqlx::query_as::<_, Webhook>("SELECT * FROM webhooks WHERE id = $1 AND account_id = $2")
            .bind(webhook_id)
            .bind(account_id)
            .fetch_one(conn)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, webhook_id = %webhook_id, "Webhook not found");
                match e {
                    sqlx::Error::RowNotFound => {
                        ServiceError::WebhookNotFound(webhook_id.to_string())
                    }
                    _ => ServiceError::DatabaseError(e.to_string()),
                }
            })?;

    Ok(webhook)
}

/// Delete a webhook
pub async fn delete_webhook(
    webhook_id: Uuid,
    account_id: Uuid,
    conn: &mut PgConnection,
) -> Result<(), ServiceError> {
    let result = sqlx::query("DELETE FROM webhooks WHERE id = $1 AND account_id = $2")
        .bind(webhook_id)
        .bind(account_id)
        .execute(conn)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, webhook_id = %webhook_id, "Failed to delete webhook");
            ServiceError::DatabaseError(e.to_string())
        })?;

    if result.rows_affected() == 0 {
        return Err(ServiceError::WebhookNotFound(webhook_id.to_string()));
    }

    tracing::info!(
        webhook_id = %webhook_id,
        account_id = %account_id,
        "Webhook deleted successfully"
    );

    Ok(())
}

/// Update webhook status (activate/deactivate)
pub async fn update_webhook_status(
    webhook_id: Uuid,
    account_id: Uuid,
    status: &str,
    conn: &mut PgConnection,
) -> Result<Webhook, ServiceError> {
    let webhook = sqlx::query_as::<_, Webhook>(
        r#"
        UPDATE webhooks 
        SET status = $1, updated_at = NOW()
        WHERE id = $2 AND account_id = $3
        RETURNING *
        "#,
    )
    .bind(status)
    .bind(webhook_id)
    .bind(account_id)
    .fetch_one(conn)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, webhook_id = %webhook_id, "Failed to update webhook");
        match e {
            sqlx::Error::RowNotFound => ServiceError::WebhookNotFound(webhook_id.to_string()),
            _ => ServiceError::DatabaseError(e.to_string()),
        }
    })?;

    tracing::info!(
        webhook_id = %webhook_id,
        status = %status,
        "Webhook status updated"
    );

    Ok(webhook)
}
