use payments_backend_dodo::datalayer::CRUD::accounts::AccountBuilder;
use payments_backend_dodo::datalayer::CRUD::api_key::ApiKeyBuilder;
use payments_backend_dodo::datalayer::CRUD::rate_limiter::RateLimiter;
use payments_backend_dodo::datalayer::db_ops::db_ops::initialize_database;
use payments_backend_dodo::errors::errors::ServiceError;
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Helper function to hash an API key
fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Helper function to get key prefix (first 8 characters)
fn get_key_prefix(key: &str) -> String {
    key.chars().take(8).collect()
}

#[tokio::test]
async fn test_rate_limiter_soft_limit() {
    println!("\n=== TEST: Rate Limiter Soft Limit (Backoff) ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL not set");
        return;
    }

    let db_ops = initialize_database()
        .await
        .expect("Failed to initialize database");
    let mut conn = db_ops
        .tracker()
        .get_connection()
        .await
        .expect("Failed to get connection");

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Rate Limit Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Rate Limit Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("Rate Limit Test Account {}", Uuid::new_v4()))
        .email(format!("ratelimit_{}@example.com", Uuid::new_v4()))
        .currency("USD".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    // Create API key
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Rate Limit Test Key".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_created_at()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_permissions()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created");

    let limiter = RateLimiter::new();

    // Test requests below soft limit (should be instant)
    println!("\n📊 Testing below soft limit (0-4 requests)...");
    for count in 0..5 {
        let start = std::time::Instant::now();
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, count)
            .await;
        let elapsed = start.elapsed();

        if let Err(ref e) = result {
            println!("❌ Request {} failed with error: {:?}", count, e);
        }

        assert!(
            result.is_ok(),
            "Request {} should succeed, got error: {:?}",
            count,
            result.err()
        );
        assert!(
            elapsed.as_millis() < 50,
            "Request {} should be instant (took {:?})",
            count,
            elapsed
        );
        println!("   Request {}: ✅ allowed instantly ({:?})", count, elapsed);
    }

    // Test requests at soft limit (should have backoff)
    println!("\n⏳ Testing at soft limit (5-9 requests, should have backoff)...");
    for count in 5..10 {
        let start = std::time::Instant::now();
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, count)
            .await;
        let elapsed = start.elapsed();

        if let Err(ref e) = result {
            println!("❌ Request {} failed with error: {:?}", count, e);
        }

        assert!(
            result.is_ok(),
            "Request {} should succeed, got error: {:?}",
            count,
            result.err()
        );
        assert!(
            elapsed.as_millis() >= 10,
            "Request {} should have backoff delay (took {:?})",
            count,
            elapsed
        );
        println!(
            "   Request {}: ✅ allowed after backoff ({:?})",
            count, elapsed
        );
    }

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(account.id)
        .execute(&mut *conn)
        .await;

    db_ops.tracker().return_connection(conn);
    db_ops.shutdown().await;

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
}

#[tokio::test]
async fn test_rate_limiter_hard_limit() {
    println!("\n=== TEST: Rate Limiter Hard Limit (Block) ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL not set");
        return;
    }

    let db_ops = initialize_database()
        .await
        .expect("Failed to initialize database");
    let mut conn = db_ops
        .tracker()
        .get_connection()
        .await
        .expect("Failed to get connection");

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Hard Limit Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Hard Limit Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("Hard Limit Test Account {}", Uuid::new_v4()))
        .email(format!("hardlimit_{}@example.com", Uuid::new_v4()))
        .currency("USD".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    // Create API key
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Hard Limit Test Key".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_created_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created");

    let limiter = RateLimiter::new();

    // Test at hard limit (should be blocked)
    println!("\n🚫 Testing at hard limit (15+ requests, should be blocked)...");
    for count in [15, 20, 100] {
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, count)
            .await;

        assert!(
            result.is_err(),
            "Request at count {} should be blocked",
            count
        );

        match result {
            Err(ServiceError::RateLimitExceeded {
                limit,
                window,
                reset_at: _,
            }) => {
                println!(
                    "   Request {}: ✅ blocked (limit: {}, window: {})",
                    count, limit, window
                );
                assert_eq!(limit, 15);
                assert_eq!(window, "total");
            }
            _ => panic!("Expected RateLimitExceeded error"),
        }
    }

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(account.id)
        .execute(&mut *conn)
        .await;

    db_ops.tracker().return_connection(conn);
    db_ops.shutdown().await;

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
}

#[tokio::test]
async fn test_rate_limiter_custom_config() {
    println!("\n=== TEST: Rate Limiter Custom Configuration ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL not set");
        return;
    }

    let db_ops = initialize_database()
        .await
        .expect("Failed to initialize database");
    let mut conn = db_ops
        .tracker()
        .get_connection()
        .await
        .expect("Failed to get connection");

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Custom Config Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Custom Config Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("Custom Config Test Account {}", Uuid::new_v4()))
        .email(format!("customconfig_{}@example.com", Uuid::new_v4()))
        .currency("USD".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    // Create API key
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Custom Config Test Key".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_created_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created");

    // Custom config: soft=10, hard=30
    let limiter = RateLimiter::with_config(10, 30, 50, 2000);

    println!("\n📊 Testing custom soft limit (10)...");
    // Below soft limit - should be instant
    let result = limiter.check_with_backoff(api_key.id, &key_prefix, 9).await;
    assert!(result.is_ok());
    println!("   Request 9: ✅ allowed instantly");

    // At soft limit - should have backoff
    let start = std::time::Instant::now();
    let result = limiter
        .check_with_backoff(api_key.id, &key_prefix, 10)
        .await;
    let elapsed = start.elapsed();
    assert!(result.is_ok());
    assert!(elapsed.as_millis() >= 10);
    println!("   Request 10: ✅ allowed with backoff ({:?})", elapsed);

    println!("\n🚫 Testing custom hard limit (30)...");
    // At hard limit - should be blocked
    let result = limiter
        .check_with_backoff(api_key.id, &key_prefix, 30)
        .await;
    assert!(result.is_err());
    match result {
        Err(ServiceError::RateLimitExceeded { limit, .. }) => {
            println!("   Request 30: ✅ blocked (limit: {})", limit);
            assert_eq!(limit, 30);
        }
        _ => panic!("Expected RateLimitExceeded error"),
    }

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(account.id)
        .execute(&mut *conn)
        .await;

    db_ops.tracker().return_connection(conn);
    db_ops.shutdown().await;

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
}
