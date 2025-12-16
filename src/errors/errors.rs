use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Main error type for the payment service
#[derive(Debug)]
pub enum ServiceError {
    // Authentication & Authorization Errors
    InvalidApiKey,
    MissingApiKey,
    ApiKeyExpired,
    InsufficientPermissions,

    // Account Errors
    AccountNotFound(String),
    AccountAlreadyExists(String),
    AccountInactive(String),
    InvalidAccountId,

    // Transaction Errors
    InsufficientBalance {
        account_id: String,
        required: i64,
        available: i64,
    },
    InvalidTransactionAmount,
    TransactionNotFound(String),
    DuplicateTransaction(String), // For idempotency key conflicts
    InvalidTransactionType,
    SameAccountTransfer,
    TransactionFailed(String),

    // Idempotency Errors
    IdempotencyKeyMismatch {
        key: String,
        reason: String,
    },

    // Webhook Errors
    WebhookNotFound(String),
    WebhookDeliveryFailed {
        webhook_id: String,
        reason: String,
    },
    InvalidWebhookUrl(String),
    WebhookAlreadyExists(String),

    // Rate Limiting
    RateLimitExceeded {
        retry_after: u64, // seconds
    },

    // Database Errors
    DatabaseError(String),
    DatabaseConnectionError,
    TransactionConflict, // For optimistic locking failures

    // Validation Errors
    ValidationError(String),
    InvalidInput(String),
    MissingRequiredField(String),

    // Internal Errors
    InternalServerError(String),
    ConfigurationError(String),

    // External Service Errors
    ExternalServiceError {
        service: String,
        reason: String,
    },
}

/// Error response structure sent to clients
#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::InvalidApiKey => write!(f, "Invalid API key provided"),
            ServiceError::MissingApiKey => write!(f, "API key is required"),
            ServiceError::ApiKeyExpired => write!(f, "API key has expired"),
            ServiceError::InsufficientPermissions => {
                write!(f, "Insufficient permissions for this operation")
            }

            ServiceError::AccountNotFound(id) => write!(f, "Account not found: {}", id),
            ServiceError::AccountAlreadyExists(id) => write!(f, "Account already exists: {}", id),
            ServiceError::AccountInactive(id) => write!(f, "Account is inactive: {}", id),
            ServiceError::InvalidAccountId => write!(f, "Invalid account ID format"),

            ServiceError::InsufficientBalance {
                account_id,
                required,
                available,
            } => {
                write!(
                    f,
                    "Insufficient balance in account {}: required {}, available {}",
                    account_id, required, available
                )
            }
            ServiceError::InvalidTransactionAmount => {
                write!(f, "Transaction amount must be positive")
            }
            ServiceError::TransactionNotFound(id) => write!(f, "Transaction not found: {}", id),
            ServiceError::DuplicateTransaction(key) => {
                write!(f, "Duplicate transaction with idempotency key: {}", key)
            }
            ServiceError::InvalidTransactionType => write!(f, "Invalid transaction type"),
            ServiceError::SameAccountTransfer => write!(f, "Cannot transfer to the same account"),
            ServiceError::TransactionFailed(reason) => write!(f, "Transaction failed: {}", reason),

            ServiceError::IdempotencyKeyMismatch { key, reason } => {
                write!(f, "Idempotency key mismatch for {}: {}", key, reason)
            }

            ServiceError::WebhookNotFound(id) => write!(f, "Webhook not found: {}", id),
            ServiceError::WebhookDeliveryFailed { webhook_id, reason } => {
                write!(f, "Webhook delivery failed for {}: {}", webhook_id, reason)
            }
            ServiceError::InvalidWebhookUrl(url) => write!(f, "Invalid webhook URL: {}", url),
            ServiceError::WebhookAlreadyExists(url) => {
                write!(f, "Webhook already exists for URL: {}", url)
            }

            ServiceError::RateLimitExceeded { retry_after } => {
                write!(
                    f,
                    "Rate limit exceeded. Retry after {} seconds",
                    retry_after
                )
            }

            ServiceError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            ServiceError::DatabaseConnectionError => write!(f, "Failed to connect to database"),
            ServiceError::TransactionConflict => {
                write!(f, "Transaction conflict detected, please retry")
            }

            ServiceError::ValidationError(msg) => write!(f, "Validation error: {}", msg),
            ServiceError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            ServiceError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {}", field)
            }

            ServiceError::InternalServerError(msg) => write!(f, "Internal server error: {}", msg),
            ServiceError::ConfigurationError(msg) => write!(f, "Configuration error: {}", msg),

            ServiceError::ExternalServiceError { service, reason } => {
                write!(f, "External service error ({}): {}", service, reason)
            }
        }
    }
}

impl std::error::Error for ServiceError {}

impl ServiceError {
    /// Get the HTTP status code for this error
    pub fn status_code(&self) -> StatusCode {
        match self {
            // 401 Unauthorized
            ServiceError::InvalidApiKey
            | ServiceError::MissingApiKey
            | ServiceError::ApiKeyExpired => StatusCode::UNAUTHORIZED,

            // 403 Forbidden
            ServiceError::InsufficientPermissions => StatusCode::FORBIDDEN,

            // 404 Not Found
            ServiceError::AccountNotFound(_)
            | ServiceError::TransactionNotFound(_)
            | ServiceError::WebhookNotFound(_) => StatusCode::NOT_FOUND,

            // 409 Conflict
            ServiceError::AccountAlreadyExists(_)
            | ServiceError::DuplicateTransaction(_)
            | ServiceError::WebhookAlreadyExists(_)
            | ServiceError::TransactionConflict
            | ServiceError::IdempotencyKeyMismatch { .. } => StatusCode::CONFLICT,

            // 422 Unprocessable Entity
            ServiceError::InsufficientBalance { .. }
            | ServiceError::AccountInactive(_)
            | ServiceError::InvalidTransactionAmount
            | ServiceError::InvalidTransactionType
            | ServiceError::SameAccountTransfer
            | ServiceError::TransactionFailed(_) => StatusCode::UNPROCESSABLE_ENTITY,

            // 400 Bad Request
            ServiceError::InvalidAccountId
            | ServiceError::ValidationError(_)
            | ServiceError::InvalidInput(_)
            | ServiceError::MissingRequiredField(_)
            | ServiceError::InvalidWebhookUrl(_) => StatusCode::BAD_REQUEST,

            // 429 Too Many Requests
            ServiceError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,

            // 502 Bad Gateway
            ServiceError::ExternalServiceError { .. }
            | ServiceError::WebhookDeliveryFailed { .. } => StatusCode::BAD_GATEWAY,

            // 503 Service Unavailable
            ServiceError::DatabaseConnectionError => StatusCode::SERVICE_UNAVAILABLE,

            // 500 Internal Server Error
            ServiceError::DatabaseError(_)
            | ServiceError::InternalServerError(_)
            | ServiceError::ConfigurationError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Get the error code string for API responses
    pub fn error_code(&self) -> &'static str {
        match self {
            ServiceError::InvalidApiKey => "INVALID_API_KEY",
            ServiceError::MissingApiKey => "MISSING_API_KEY",
            ServiceError::ApiKeyExpired => "API_KEY_EXPIRED",
            ServiceError::InsufficientPermissions => "INSUFFICIENT_PERMISSIONS",

            ServiceError::AccountNotFound(_) => "ACCOUNT_NOT_FOUND",
            ServiceError::AccountAlreadyExists(_) => "ACCOUNT_ALREADY_EXISTS",
            ServiceError::AccountInactive(_) => "ACCOUNT_INACTIVE",
            ServiceError::InvalidAccountId => "INVALID_ACCOUNT_ID",

            ServiceError::InsufficientBalance { .. } => "INSUFFICIENT_BALANCE",
            ServiceError::InvalidTransactionAmount => "INVALID_TRANSACTION_AMOUNT",
            ServiceError::TransactionNotFound(_) => "TRANSACTION_NOT_FOUND",
            ServiceError::DuplicateTransaction(_) => "DUPLICATE_TRANSACTION",
            ServiceError::InvalidTransactionType => "INVALID_TRANSACTION_TYPE",
            ServiceError::SameAccountTransfer => "SAME_ACCOUNT_TRANSFER",
            ServiceError::TransactionFailed(_) => "TRANSACTION_FAILED",

            ServiceError::IdempotencyKeyMismatch { .. } => "IDEMPOTENCY_KEY_MISMATCH",

            ServiceError::WebhookNotFound(_) => "WEBHOOK_NOT_FOUND",
            ServiceError::WebhookDeliveryFailed { .. } => "WEBHOOK_DELIVERY_FAILED",
            ServiceError::InvalidWebhookUrl(_) => "INVALID_WEBHOOK_URL",
            ServiceError::WebhookAlreadyExists(_) => "WEBHOOK_ALREADY_EXISTS",

            ServiceError::RateLimitExceeded { .. } => "RATE_LIMIT_EXCEEDED",

            ServiceError::DatabaseError(_) => "DATABASE_ERROR",
            ServiceError::DatabaseConnectionError => "DATABASE_CONNECTION_ERROR",
            ServiceError::TransactionConflict => "TRANSACTION_CONFLICT",

            ServiceError::ValidationError(_) => "VALIDATION_ERROR",
            ServiceError::InvalidInput(_) => "INVALID_INPUT",
            ServiceError::MissingRequiredField(_) => "MISSING_REQUIRED_FIELD",

            ServiceError::InternalServerError(_) => "INTERNAL_SERVER_ERROR",
            ServiceError::ConfigurationError(_) => "CONFIGURATION_ERROR",

            ServiceError::ExternalServiceError { .. } => "EXTERNAL_SERVICE_ERROR",
        }
    }

    /// Convert error to JSON details for response
    pub fn to_details(&self) -> Option<serde_json::Value> {
        match self {
            ServiceError::InsufficientBalance {
                account_id,
                required,
                available,
            } => Some(serde_json::json!({
                "account_id": account_id,
                "required_amount": required,
                "available_amount": available
            })),
            ServiceError::RateLimitExceeded { retry_after } => Some(serde_json::json!({
                "retry_after_seconds": retry_after
            })),
            ServiceError::IdempotencyKeyMismatch { key, reason } => Some(serde_json::json!({
                "idempotency_key": key,
                "reason": reason
            })),
            ServiceError::WebhookDeliveryFailed { webhook_id, reason } => Some(serde_json::json!({
                "webhook_id": webhook_id,
                "reason": reason
            })),
            ServiceError::ExternalServiceError { service, reason } => Some(serde_json::json!({
                "service": service,
                "reason": reason
            })),
            _ => None,
        }
    }
}

/// Implement IntoResponse for Axum integration
impl IntoResponse for ServiceError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_response = ErrorResponse {
            error: ErrorDetail {
                code: self.error_code().to_string(),
                message: self.to_string(),
                details: self.to_details(),
            },
        };

        (status, Json(error_response)).into_response()
    }
}

/// Conversion from sqlx errors
#[cfg(feature = "sqlx")]
impl From<sqlx::Error> for ServiceError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => ServiceError::DatabaseError("Record not found".to_string()),
            sqlx::Error::PoolTimedOut => ServiceError::DatabaseConnectionError,
            _ => ServiceError::DatabaseError(err.to_string()),
        }
    }
}

/// Conversion from validation errors
impl From<validator::ValidationErrors> for ServiceError {
    fn from(err: validator::ValidationErrors) -> Self {
        ServiceError::ValidationError(err.to_string())
    }
}

/// Type alias for Results using ServiceError
pub type ServiceResult<T> = Result<T, ServiceError>;
