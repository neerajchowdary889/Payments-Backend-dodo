#[cfg(test)]
mod error_tests {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    use payments_backend_dodo::errors::errors::{
        ErrorDetail, ErrorResponse, ServiceError, ServiceResult,
    };
    use serde_json;

    // Test error display messages
    #[test]
    fn test_error_display_messages() {
        let error = ServiceError::InvalidApiKey;
        assert_eq!(error.to_string(), "Invalid API key provided");

        let error = ServiceError::AccountNotFound("acc_123".to_string());
        assert_eq!(error.to_string(), "Account not found: acc_123");

        let error = ServiceError::InsufficientBalance {
            account_id: "acc_456".to_string(),
            required: 1000,
            available: 500,
        };
        assert_eq!(
            error.to_string(),
            "Insufficient balance in account acc_456: required 1000, available 500"
        );

        let error = ServiceError::RateLimitExceeded { retry_after: 60 };
        assert_eq!(
            error.to_string(),
            "Rate limit exceeded. Retry after 60 seconds"
        );
    }

    // Test HTTP status codes
    #[test]
    fn test_authentication_error_status_codes() {
        assert_eq!(
            ServiceError::InvalidApiKey.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ServiceError::MissingApiKey.status_code(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ServiceError::ApiKeyExpired.status_code(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn test_authorization_error_status_codes() {
        assert_eq!(
            ServiceError::InsufficientPermissions.status_code(),
            StatusCode::FORBIDDEN
        );
    }

    #[test]
    fn test_not_found_error_status_codes() {
        assert_eq!(
            ServiceError::AccountNotFound("acc_123".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ServiceError::TransactionNotFound("txn_456".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ServiceError::WebhookNotFound("wh_789".to_string()).status_code(),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn test_conflict_error_status_codes() {
        assert_eq!(
            ServiceError::AccountAlreadyExists("acc_123".to_string()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ServiceError::DuplicateTransaction("idem_key_123".to_string()).status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ServiceError::TransactionConflict.status_code(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ServiceError::IdempotencyKeyMismatch {
                key: "key_123".to_string(),
                reason: "Different payload".to_string(),
            }
            .status_code(),
            StatusCode::CONFLICT
        );
    }

    #[test]
    fn test_unprocessable_entity_status_codes() {
        assert_eq!(
            ServiceError::InsufficientBalance {
                account_id: "acc_123".to_string(),
                required: 1000,
                available: 500,
            }
            .status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ServiceError::InvalidTransactionAmount.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            ServiceError::SameAccountTransfer.status_code(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }

    #[test]
    fn test_bad_request_status_codes() {
        assert_eq!(
            ServiceError::InvalidAccountId.status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ServiceError::ValidationError("Invalid format".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ServiceError::MissingRequiredField("amount".to_string()).status_code(),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn test_rate_limit_status_code() {
        assert_eq!(
            ServiceError::RateLimitExceeded { retry_after: 60 }.status_code(),
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    #[test]
    fn test_server_error_status_codes() {
        assert_eq!(
            ServiceError::DatabaseError("Connection failed".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ServiceError::InternalServerError("Unexpected error".to_string()).status_code(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
        assert_eq!(
            ServiceError::DatabaseConnectionError.status_code(),
            StatusCode::SERVICE_UNAVAILABLE
        );
    }

    // Test error codes
    #[test]
    fn test_error_codes() {
        assert_eq!(ServiceError::InvalidApiKey.error_code(), "INVALID_API_KEY");
        assert_eq!(
            ServiceError::AccountNotFound("acc_123".to_string()).error_code(),
            "ACCOUNT_NOT_FOUND"
        );
        assert_eq!(
            ServiceError::InsufficientBalance {
                account_id: "acc_123".to_string(),
                required: 1000,
                available: 500,
            }
            .error_code(),
            "INSUFFICIENT_BALANCE"
        );
        assert_eq!(
            ServiceError::RateLimitExceeded { retry_after: 60 }.error_code(),
            "RATE_LIMIT_EXCEEDED"
        );
        assert_eq!(
            ServiceError::TransactionConflict.error_code(),
            "TRANSACTION_CONFLICT"
        );
    }

    // Test error details
    #[test]
    fn test_insufficient_balance_details() {
        let error = ServiceError::InsufficientBalance {
            account_id: "acc_123".to_string(),
            required: 1000,
            available: 500,
        };

        let details = error.to_details().unwrap();
        assert_eq!(details["account_id"], "acc_123");
        assert_eq!(details["required_amount"], 1000);
        assert_eq!(details["available_amount"], 500);
    }

    #[test]
    fn test_rate_limit_details() {
        let error = ServiceError::RateLimitExceeded { retry_after: 120 };

        let details = error.to_details().unwrap();
        assert_eq!(details["retry_after_seconds"], 120);
    }

    #[test]
    fn test_idempotency_key_mismatch_details() {
        let error = ServiceError::IdempotencyKeyMismatch {
            key: "idem_key_123".to_string(),
            reason: "Different request body".to_string(),
        };

        let details = error.to_details().unwrap();
        assert_eq!(details["idempotency_key"], "idem_key_123");
        assert_eq!(details["reason"], "Different request body");
    }

    #[test]
    fn test_webhook_delivery_failed_details() {
        let error = ServiceError::WebhookDeliveryFailed {
            webhook_id: "wh_123".to_string(),
            reason: "Connection timeout".to_string(),
        };

        let details = error.to_details().unwrap();
        assert_eq!(details["webhook_id"], "wh_123");
        assert_eq!(details["reason"], "Connection timeout");
    }

    #[test]
    fn test_external_service_error_details() {
        let error = ServiceError::ExternalServiceError {
            service: "payment_gateway".to_string(),
            reason: "Gateway timeout".to_string(),
        };

        let details = error.to_details().unwrap();
        assert_eq!(details["service"], "payment_gateway");
        assert_eq!(details["reason"], "Gateway timeout");
    }

    #[test]
    fn test_errors_without_details() {
        assert!(ServiceError::InvalidApiKey.to_details().is_none());
        assert!(
            ServiceError::AccountNotFound("acc_123".to_string())
                .to_details()
                .is_none()
        );
        assert!(ServiceError::TransactionConflict.to_details().is_none());
    }

    // Test Axum response integration
    #[test]
    fn test_into_response_status_code() {
        let error = ServiceError::InvalidApiKey;
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_into_response_not_found() {
        let error = ServiceError::AccountNotFound("acc_123".to_string());
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_into_response_rate_limit() {
        let error = ServiceError::RateLimitExceeded { retry_after: 60 };
        let response = error.into_response();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // Test error response serialization
    #[test]
    fn test_error_response_serialization() {
        let error_response = ErrorResponse {
            error: ErrorDetail {
                code: "INVALID_API_KEY".to_string(),
                message: "Invalid API key provided".to_string(),
                details: None,
            },
        };

        let json = serde_json::to_string(&error_response).unwrap();
        assert!(json.contains("INVALID_API_KEY"));
        assert!(json.contains("Invalid API key provided"));
    }

    #[test]
    fn test_error_response_with_details_serialization() {
        let error_response = ErrorResponse {
            error: ErrorDetail {
                code: "INSUFFICIENT_BALANCE".to_string(),
                message: "Insufficient balance".to_string(),
                details: Some(serde_json::json!({
                    "account_id": "acc_123",
                    "required_amount": 1000,
                    "available_amount": 500
                })),
            },
        };

        let json = serde_json::to_string(&error_response).unwrap();
        assert!(json.contains("INSUFFICIENT_BALANCE"));
        assert!(json.contains("acc_123"));
        assert!(json.contains("1000"));
        assert!(json.contains("500"));
    }

    #[test]
    fn test_error_response_deserialization() {
        let json = r#"{
            "error": {
                "code": "ACCOUNT_NOT_FOUND",
                "message": "Account not found: acc_123"
            }
        }"#;

        let error_response: ErrorResponse = serde_json::from_str(json).unwrap();
        assert_eq!(error_response.error.code, "ACCOUNT_NOT_FOUND");
        assert_eq!(error_response.error.message, "Account not found: acc_123");
        assert!(error_response.error.details.is_none());
    }

    // Test ServiceResult type alias
    #[test]
    fn test_service_result_ok() {
        let result: ServiceResult<i32> = Ok(42);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_service_result_err() {
        let result: ServiceResult<i32> = Err(ServiceError::InvalidApiKey);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Invalid API key provided");
    }

    // Test all account-related errors
    #[test]
    fn test_account_errors() {
        let errors = vec![
            ServiceError::AccountNotFound("acc_123".to_string()),
            ServiceError::AccountAlreadyExists("acc_456".to_string()),
            ServiceError::AccountInactive("acc_789".to_string()),
            ServiceError::InvalidAccountId,
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
            assert!(!error.error_code().is_empty());
        }
    }

    // Test all transaction-related errors
    #[test]
    fn test_transaction_errors() {
        let errors = vec![
            ServiceError::InsufficientBalance {
                account_id: "acc_123".to_string(),
                required: 1000,
                available: 500,
            },
            ServiceError::InvalidTransactionAmount,
            ServiceError::TransactionNotFound("txn_123".to_string()),
            ServiceError::DuplicateTransaction("idem_key_123".to_string()),
            ServiceError::InvalidTransactionType,
            ServiceError::SameAccountTransfer,
            ServiceError::TransactionFailed("Network error".to_string()),
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
            assert!(!error.error_code().is_empty());
        }
    }

    // Test all webhook-related errors
    #[test]
    fn test_webhook_errors() {
        let errors = vec![
            ServiceError::WebhookNotFound("wh_123".to_string()),
            ServiceError::WebhookDeliveryFailed {
                webhook_id: "wh_456".to_string(),
                reason: "Timeout".to_string(),
            },
            ServiceError::InvalidWebhookUrl("invalid-url".to_string()),
            ServiceError::WebhookAlreadyExists("https://example.com/webhook".to_string()),
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
            assert!(!error.error_code().is_empty());
        }
    }

    // Test validation errors
    #[test]
    fn test_validation_errors() {
        let errors = vec![
            ServiceError::ValidationError("Invalid email format".to_string()),
            ServiceError::InvalidInput("Amount must be positive".to_string()),
            ServiceError::MissingRequiredField("account_id".to_string()),
        ];

        for error in errors {
            assert_eq!(error.status_code(), StatusCode::BAD_REQUEST);
            assert!(!error.to_string().is_empty());
        }
    }

    // Test database errors
    #[test]
    fn test_database_errors() {
        let db_error = ServiceError::DatabaseError("Connection lost".to_string());
        assert_eq!(db_error.status_code(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(db_error.error_code(), "DATABASE_ERROR");

        let conn_error = ServiceError::DatabaseConnectionError;
        assert_eq!(conn_error.status_code(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(conn_error.error_code(), "DATABASE_CONNECTION_ERROR");

        let conflict_error = ServiceError::TransactionConflict;
        assert_eq!(conflict_error.status_code(), StatusCode::CONFLICT);
        assert_eq!(conflict_error.error_code(), "TRANSACTION_CONFLICT");
    }

    // Test error code uniqueness
    #[test]
    fn test_error_code_uniqueness() {
        use std::collections::HashSet;

        let error_codes: Vec<&str> = vec![
            ServiceError::InvalidApiKey.error_code(),
            ServiceError::MissingApiKey.error_code(),
            ServiceError::AccountNotFound("test".to_string()).error_code(),
            ServiceError::InsufficientBalance {
                account_id: "test".to_string(),
                required: 100,
                available: 50,
            }
            .error_code(),
            ServiceError::RateLimitExceeded { retry_after: 60 }.error_code(),
            ServiceError::WebhookNotFound("test".to_string()).error_code(),
            ServiceError::TransactionConflict.error_code(),
            ServiceError::ValidationError("test".to_string()).error_code(),
        ];

        let unique_codes: HashSet<&str> = error_codes.iter().copied().collect();
        assert_eq!(
            error_codes.len(),
            unique_codes.len(),
            "Error codes must be unique"
        );
    }

    // Test that all errors implement Error trait
    #[test]
    fn test_error_trait_implementation() {
        let error: Box<dyn std::error::Error> = Box::new(ServiceError::InvalidApiKey);
        assert!(!error.to_string().is_empty());
    }

    // Test error formatting consistency
    #[test]
    fn test_error_message_formatting() {
        let error = ServiceError::AccountNotFound("acc_123".to_string());
        let display = format!("{}", error);
        let debug = format!("{:?}", error);

        assert!(display.contains("acc_123"));
        assert!(debug.contains("AccountNotFound"));
    }
}
