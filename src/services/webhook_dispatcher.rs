use crate::datalayer::{
    CRUD::{types::Transaction, webhook::Webhook},
    db_ops::constants::POOL_STATE_TRACKER,
};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;
use std::time::Duration;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Simple webhook dispatcher - sends webhooks asynchronously
pub struct WebhookDispatcher {
    client: reqwest::Client,
}

impl WebhookDispatcher {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self { client }
    }

    /// Dispatch webhook for debit transaction (amount debited from account)
    pub fn dispatch_debit_webhook(&self, webhook: Webhook, transaction: Transaction) {
        let client = self.client.clone();
        let webhook = webhook.clone();
        let transaction = transaction.clone();

        // Spawn async task (fire and forget)
        tokio::spawn(async move {
            let payload = json!({
                "event": "transaction.debited",
                "message": "Amount has been debited from your account",
                "data": {
                    "transaction_id": transaction.id,
                    "amount": transaction.amount,
                    "currency": transaction.currency,
                    "description": transaction.description,
                    "parent_tx_key": transaction.parent_tx_key,
                },
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) =
                Self::send_webhook(&client, &webhook, &payload, "transaction.debited").await
            {
                tracing::error!(
                    webhook_id = %webhook.id,
                    url = %webhook.url,
                    error = %e,
                    "Failed to send debit webhook"
                );
            }
        });
    }

    /// Dispatch webhook for credit transaction (amount credited to account)
    pub fn dispatch_credit_webhook(&self, webhook: Webhook, transaction: Transaction) {
        let client = self.client.clone();
        let webhook = webhook.clone();
        let transaction = transaction.clone();

        // Spawn async task (fire and forget)
        tokio::spawn(async move {
            let payload = json!({
                "event": "transaction.credited",
                "message": "Amount has been credited to your account",
                "data": {
                    "transaction_id": transaction.id,
                    "amount": transaction.amount,
                    "currency": transaction.currency,
                    "description": transaction.description,
                    "parent_tx_key": transaction.parent_tx_key,
                },
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            if let Err(e) =
                Self::send_webhook(&client, &webhook, &payload, "transaction.credited").await
            {
                tracing::error!(
                    webhook_id = %webhook.id,
                    url = %webhook.url,
                    error = %e,
                    "Failed to send credit webhook"
                );
            }
        });
    }

    async fn send_webhook(
        client: &reqwest::Client,
        webhook: &Webhook,
        payload: &serde_json::Value,
        event_type: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload_str = serde_json::to_string(payload)?;

        // Generate HMAC signature
        let signature = Self::generate_signature(&payload_str, &webhook.secret);

        tracing::info!(
            webhook_id = %webhook.id,
            url = %webhook.url,
            event = %event_type,
            "Sending webhook"
        );

        // Extract transaction_id from payload if present
        let transaction_id = payload["data"]["transaction_id"]
            .as_str()
            .and_then(|s| Uuid::parse_str(s).ok());

        // Send HTTP POST
        let response_result = client
            .post(&webhook.url)
            .header("Content-Type", "application/json")
            .header("X-Webhook-Signature", signature)
            .header("X-Webhook-Event", event_type)
            .body(payload_str.clone())
            .send()
            .await;

        // Log delivery attempt
        match response_result {
            Ok(response) => {
                let status_code = response.status().as_u16() as i32;
                let is_success = response.status().is_success();
                let response_body = response.text().await.ok();

                // Log to database
                Self::log_delivery(
                    webhook.id,
                    transaction_id,
                    event_type,
                    payload.clone(),
                    if is_success { "delivered" } else { "failed" },
                    Some(status_code),
                    response_body.as_deref(),
                    None,
                )
                .await;

                if is_success {
                    tracing::info!(
                        webhook_id = %webhook.id,
                        status = %status_code,
                        "Webhook delivered successfully"
                    );
                } else {
                    tracing::warn!(
                        webhook_id = %webhook.id,
                        status = %status_code,
                        "Webhook delivery failed"
                    );
                }

                Ok(())
            }
            Err(e) => {
                // Log failed delivery
                Self::log_delivery(
                    webhook.id,
                    transaction_id,
                    event_type,
                    payload.clone(),
                    "failed",
                    None,
                    None,
                    Some(&e.to_string()),
                )
                .await;

                tracing::error!(
                    webhook_id = %webhook.id,
                    error = %e,
                    "Webhook delivery error"
                );

                Err(Box::new(e))
            }
        }
    }

    /// Log webhook delivery attempt to database
    async fn log_delivery(
        webhook_id: Uuid,
        transaction_id: Option<Uuid>,
        event_type: &str,
        payload: serde_json::Value,
        status: &str,
        http_status_code: Option<i32>,
        response_body: Option<&str>,
        error_message: Option<&str>,
    ) {
        let tracker = match POOL_STATE_TRACKER.get() {
            Some(t) => t,
            None => {
                tracing::error!("Failed to get pool tracker for delivery logging");
                return;
            }
        };

        let mut conn = match tracker.get_connection().await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!(error = %e, "Failed to get connection for delivery logging");
                return;
            }
        };

        let delivered_at = if status == "delivered" {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let failed_at = if status == "failed" {
            Some(chrono::Utc::now())
        } else {
            None
        };

        let result = sqlx::query(
            r#"
            INSERT INTO webhook_deliveries (
                webhook_id, transaction_id, event_type, payload, status,
                attempt_count, http_status_code, response_body, error_message,
                delivered_at, failed_at
            )
            VALUES ($1, $2, $3, $4, $5, 1, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(webhook_id)
        .bind(transaction_id)
        .bind(event_type)
        .bind(payload)
        .bind(status)
        .bind(http_status_code)
        .bind(response_body)
        .bind(error_message)
        .bind(delivered_at)
        .bind(failed_at)
        .execute(&mut *conn)
        .await;

        tracker.return_connection(conn);

        match result {
            Ok(_) => {
                tracing::debug!(
                    webhook_id = %webhook_id,
                    status = %status,
                    "Webhook delivery logged"
                );
            }
            Err(e) => {
                tracing::error!(
                    error = %e,
                    webhook_id = %webhook_id,
                    "Failed to log webhook delivery"
                );
            }
        }
    }

    fn generate_signature(payload: &str, secret: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload.as_bytes());
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }
}

impl Default for WebhookDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
