use payments_backend_dodo::datalayer::CRUD::accounts::AccountBuilder;
use payments_backend_dodo::datalayer::CRUD::api_key::ApiKeyBuilder;
use payments_backend_dodo::datalayer::CRUD::rate_limiter::RateLimiter;
use payments_backend_dodo::datalayer::initialize_database;
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
async fn test_rate_limiter_with_redis_soft_limit() {
    println!("\n=== TEST: Rate Limiter with Redis - Soft Limit (Backoff) ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL not set");
        return;
    }

    if std::env::var("REDIS_URL").is_err() {
        println!("⚠️  Skipping test: REDIS_URL not set");
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

    // Initialize Redis
    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

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
        .expect_permissions()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_created_at()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created");

    let limiter = RateLimiter::new();
    let endpoint = "/api/v1/test";

    // Clear any existing Redis counters
    let _ = limiter
        .reset_count(api_key.id, endpoint, redis_conn.clone())
        .await;

    println!("\n📊 Testing below soft limit (0-4 requests)...");
    for i in 0..5 {
        let start = std::time::Instant::now();
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, endpoint, redis_conn.clone())
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Request {} should succeed", i);
        println!("   Request {}: ✅ allowed instantly ({:?})", i, elapsed);
    }

    println!("\n⏳ Testing at soft limit (5-9 requests, should have backoff)...");
    for i in 5..10 {
        let start = std::time::Instant::now();
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, endpoint, redis_conn.clone())
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Request {} should succeed", i);
        println!("   Request {}: ✅ allowed after backoff ({:?})", i, elapsed);
        assert!(elapsed.as_millis() >= 100, "Should have backoff delay");
    }

    // Cleanup
    let _ = limiter
        .reset_count(api_key.id, endpoint, redis_conn.clone())
        .await;
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
async fn test_rate_limiter_with_redis_hard_limit() {
    println!("\n=== TEST: Rate Limiter with Redis - Hard Limit (Block) ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() || std::env::var("REDIS_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL or REDIS_URL not set");
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

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

    // Create test account and API key
    let account = AccountBuilder::new()
        .business_name(format!("Hard Limit Test {}", Uuid::new_v4()))
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

    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash)
        .key_prefix(key_prefix.clone())
        .name("Hard Limit Test Key".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_permissions()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_created_at()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created");

    let limiter = RateLimiter::new();
    let endpoint = "/api/v1/test_hard";

    // Clear Redis counter
    let _ = limiter
        .reset_count(api_key.id, endpoint, redis_conn.clone())
        .await;

    println!("\n🚫 Testing hard limit (15 requests)...");

    // Make 14 requests (should all succeed)
    for i in 0..14 {
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, endpoint, redis_conn.clone())
            .await;
        assert!(result.is_ok(), "Request {} should succeed", i);
    }

    // 15th request should be blocked
    let result = limiter
        .check_with_backoff(api_key.id, &key_prefix, endpoint, redis_conn.clone())
        .await;

    assert!(result.is_err(), "Request 15 should be blocked");
    match result {
        Err(ServiceError::RateLimitExceeded { limit, window, .. }) => {
            println!(
                "   Request 15: ✅ blocked (limit: {}, window: {})",
                limit, window
            );
            assert_eq!(limit, 15);
            assert_eq!(window, endpoint);
        }
        _ => panic!("Expected RateLimitExceeded error"),
    }

    // Cleanup
    let _ = limiter.reset_count(api_key.id, endpoint, redis_conn).await;
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
async fn test_rate_limiter_per_endpoint_isolation() {
    println!("\n=== TEST: Rate Limiter - Per-Endpoint Isolation ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() || std::env::var("REDIS_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL or REDIS_URL not set");
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

    let redis_url =
        std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
    let redis_client = redis::Client::open(redis_url).expect("Failed to create Redis client");
    let redis_conn = redis::aio::ConnectionManager::new(redis_client)
        .await
        .expect("Failed to connect to Redis");

    // Create test account and API key
    let account = AccountBuilder::new()
        .business_name(format!("Endpoint Test {}", Uuid::new_v4()))
        .email(format!("endpoint_{}@example.com", Uuid::new_v4()))
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

    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash)
        .key_prefix(key_prefix.clone())
        .name("Endpoint Test Key".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_permissions()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_created_at()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created");

    let limiter = RateLimiter::new();
    let endpoint1 = "/api/v1/accounts";
    let endpoint2 = "/api/v1/transfer";

    // Clear Redis counters
    let _ = limiter
        .reset_count(api_key.id, endpoint1, redis_conn.clone())
        .await;
    let _ = limiter
        .reset_count(api_key.id, endpoint2, redis_conn.clone())
        .await;

    println!("\n📊 Testing endpoint isolation...");

    // Make 10 requests to endpoint1
    for i in 0..10 {
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, endpoint1, redis_conn.clone())
            .await;
        assert!(result.is_ok(), "Endpoint1 request {} should succeed", i);
    }
    println!("   ✅ Made 10 requests to {}", endpoint1);

    // Endpoint2 should still have 0 count - first 5 requests should be instant
    for i in 0..5 {
        let start = std::time::Instant::now();
        let result = limiter
            .check_with_backoff(api_key.id, &key_prefix, endpoint2, redis_conn.clone())
            .await;
        let elapsed = start.elapsed();

        assert!(result.is_ok(), "Endpoint2 request {} should succeed", i);
        assert!(elapsed.as_millis() < 100, "Should be instant (no backoff)");
    }
    println!(
        "   ✅ First 5 requests to {} were instant (independent counter)",
        endpoint2
    );

    // Verify counts
    let count1 = limiter
        .get_count(api_key.id, endpoint1, redis_conn.clone())
        .await
        .unwrap();
    let count2 = limiter
        .get_count(api_key.id, endpoint2, redis_conn.clone())
        .await
        .unwrap();

    println!("\n📈 Final counts:");
    println!("   {} count: {}", endpoint1, count1);
    println!("   {} count: {}", endpoint2, count2);

    assert_eq!(count1, 10, "Endpoint1 should have 10 requests");
    assert_eq!(count2, 5, "Endpoint2 should have 5 requests");

    // Cleanup
    let _ = limiter
        .reset_count(api_key.id, endpoint1, redis_conn.clone())
        .await;
    let _ = limiter.reset_count(api_key.id, endpoint2, redis_conn).await;
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
    println!("✅ Verified: Each endpoint has independent rate limiting!");
}
