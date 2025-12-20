use payments_backend_dodo::datalayer::CRUD::accounts::AccountBuilder;
use payments_backend_dodo::datalayer::CRUD::api_key::ApiKeyBuilder;
use payments_backend_dodo::datalayer::CRUD::rate_limiter::{RateLimiter, WindowType};
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
async fn test_rate_limit_enforcement_per_minute() {
    println!("\n=== TEST: Rate Limit Enforcement (Per Minute) ===");

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
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE TRUE")
        .execute(&mut *conn)
        .await;
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

    // Create API key with rate limit of 5 requests per minute
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Rate Limit Test Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(5)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_rate_limit_per_minute()
        .expect_rate_limit_per_hour()
        .expect_permissions()
        .expect_created_at()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created with rate_limit_per_minute = 5");

    // Make 5 requests - all should succeed
    println!("\n📊 Making 5 requests (should all succeed)...");
    for i in 1..=5 {
        let result = RateLimiter::check_rate_limit(api_key.id, &key_prefix).await;

        if let Err(ref e) = result {
            println!("❌ Request {} failed with error: {:?}", i, e);
        }

        assert!(
            result.is_ok(),
            "Request {} should succeed, but got error: {:?}",
            i,
            result.err()
        );

        if let Ok(results) = result {
            for r in results {
                if r.window_type == WindowType::Minute {
                    println!("   Request {}: {} remaining", i, r.remaining);
                    assert_eq!(r.limit, 5);
                    assert_eq!(r.remaining, 5 - i as i32);
                }
            }
        }
    }

    // 6th request should fail
    println!("\n🚫 Making 6th request (should fail)...");
    let result = RateLimiter::check_rate_limit(api_key.id, &key_prefix).await;
    assert!(result.is_err(), "6th request should fail");

    match result {
        Err(ServiceError::RateLimitExceeded {
            limit,
            window,
            reset_at,
        }) => {
            println!("✅ Rate limit exceeded as expected:");
            println!("   - Limit: {}", limit);
            println!("   - Window: {}", window);
            println!("   - Reset at: {}", reset_at);
            assert_eq!(limit, 5);
            assert_eq!(window, "minute");
        }
        _ => panic!("Expected RateLimitExceeded error"),
    }

    // Cleanup
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE api_key_id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
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
async fn test_rate_limit_multiple_windows() {
    println!("\n=== TEST: Multiple Window Types (Minute + Hour) ===");

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
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE TRUE")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Multi Window Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Multi Window Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("Multi Window Test Account {}", Uuid::new_v4()))
        .email(format!("multiwindow_{}@example.com", Uuid::new_v4()))
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

    // Create API key with both minute and hour limits
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Multi Window Test Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(3)
        .rate_limit_per_hour(10)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_rate_limit_per_minute()
        .expect_rate_limit_per_hour()
        .expect_created_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created:");
    println!("   - rate_limit_per_minute = 3");
    println!("   - rate_limit_per_hour = 10");

    // Make 3 requests - all should succeed
    println!("\n📊 Making 3 requests...");
    for i in 1..=3 {
        let result = RateLimiter::check_rate_limit(api_key.id, &key_prefix).await;
        assert!(result.is_ok(), "Request {} should succeed", i);

        if let Ok(results) = result {
            println!("   Request {}:", i);
            for r in results {
                match r.window_type {
                    WindowType::Minute => println!("     - Minute: {} remaining", r.remaining),
                    WindowType::Hour => println!("     - Hour: {} remaining", r.remaining),
                }
            }
        }
    }

    // 4th request should fail on minute limit
    println!("\n🚫 Making 4th request (should fail on minute limit)...");
    let result = RateLimiter::check_rate_limit(api_key.id, &key_prefix).await;
    assert!(result.is_err(), "4th request should fail");

    match result {
        Err(ServiceError::RateLimitExceeded { limit, window, .. }) => {
            println!("✅ Rate limit exceeded as expected:");
            println!("   - Limit: {}", limit);
            println!("   - Window: {}", window);
            assert_eq!(limit, 3);
            assert_eq!(window, "minute");
        }
        _ => panic!("Expected RateLimitExceeded error"),
    }

    // Cleanup
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE api_key_id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
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
async fn test_rate_limit_status_without_increment() {
    println!("\n=== TEST: Get Status Without Incrementing ===");

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
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE TRUE")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Status Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Status Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("Status Test Account {}", Uuid::new_v4()))
        .email(format!("status_{}@example.com", Uuid::new_v4()))
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
        .name("Status Test Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(10)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_rate_limit_per_minute()
        .expect_created_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created with rate_limit_per_minute = 10");

    // Make 3 requests
    println!("\n📊 Making 3 requests...");
    for i in 1..=3 {
        RateLimiter::check_rate_limit(api_key.id, &key_prefix)
            .await
            .expect("Request should succeed");
        println!("   Request {} completed", i);
    }

    // Check status without incrementing
    println!("\n🔍 Checking status without incrementing...");
    let status = RateLimiter::get_rate_limit_status(api_key.id)
        .await
        .expect("Failed to get status");

    for result in status {
        if result.window_type == WindowType::Minute {
            println!("✅ Current status:");
            println!("   - Limit: {}", result.limit);
            println!("   - Remaining: {}", result.remaining);
            println!("   - Allowed: {}", result.allowed);

            assert_eq!(result.limit, 10);
            assert_eq!(result.remaining, 7); // 10 - 3 = 7
            assert!(result.allowed);
        }
    }

    // Check status again - should be the same (not incremented)
    println!("\n🔍 Checking status again (should be same)...");
    let status2 = RateLimiter::get_rate_limit_status(api_key.id)
        .await
        .expect("Failed to get status");

    for result in status2 {
        if result.window_type == WindowType::Minute {
            assert_eq!(result.remaining, 7); // Still 7, not decremented
            println!("✅ Status unchanged: {} remaining", result.remaining);
        }
    }

    // Cleanup
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE api_key_id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
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
async fn test_rate_limit_cleanup() {
    println!("\n=== TEST: Cleanup Old Counters ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL not set");
        return;
    }

    let db_ops = initialize_database()
        .await
        .expect("Failed to initialize database");
    let pool = db_ops.pool();
    let mut conn = db_ops
        .tracker()
        .get_connection()
        .await
        .expect("Failed to get connection");

    // Cleanup
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE TRUE")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Cleanup Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Cleanup Test%'")
        .execute(&mut *conn)
        .await;

    // Create account and API key
    let account = AccountBuilder::new()
        .business_name(format!("Cleanup Test Account {}", Uuid::new_v4()))
        .email(format!("cleanup_{}@example.com", Uuid::new_v4()))
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
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Cleanup Test Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(10)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_rate_limit_per_minute()
        .expect_created_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    // Insert old counter (3 hours ago)
    let old_time = chrono::Utc::now() - chrono::Duration::hours(3);
    sqlx::query(
        "INSERT INTO rate_limit_counters (api_key_id, window_start, window_type, request_count)
         VALUES ($1, $2, 'minute', 5)",
    )
    .bind(api_key.id)
    .bind(old_time)
    .execute(&mut *conn)
    .await
    .expect("Failed to insert old counter");

    // Insert recent counter (current time)
    let recent_time = chrono::Utc::now();
    sqlx::query(
        "INSERT INTO rate_limit_counters (api_key_id, window_start, window_type, request_count)
         VALUES ($1, $2, 'minute', 3)",
    )
    .bind(api_key.id)
    .bind(recent_time)
    .execute(&mut *conn)
    .await
    .expect("Failed to insert recent counter");

    println!("✅ Created 2 counters:");
    println!("   - Old counter (3 hours ago)");
    println!("   - Recent counter (now)");

    // Count before cleanup
    let count_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rate_limit_counters WHERE api_key_id = $1")
            .bind(api_key.id)
            .fetch_one(&mut *conn)
            .await
            .expect("Failed to count");

    println!("\n📊 Counters before cleanup: {}", count_before);
    assert_eq!(count_before, 2);

    // Run cleanup
    println!("\n🧹 Running cleanup...");
    let deleted = RateLimiter::cleanup_old_counters(pool)
        .await
        .expect("Cleanup failed");
    println!("✅ Deleted {} old counters", deleted);

    // Count after cleanup
    let count_after: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM rate_limit_counters WHERE api_key_id = $1")
            .bind(api_key.id)
            .fetch_one(&mut *conn)
            .await
            .expect("Failed to count");

    println!("📊 Counters after cleanup: {}", count_after);
    assert_eq!(count_after, 1); // Only recent counter should remain

    // Cleanup
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE api_key_id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
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
async fn test_rate_limit_revoked_api_key() {
    println!("\n=== TEST: Revoked API Key Should Fail ===");

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
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE TRUE")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Revoked Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Revoked Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("Revoked Test Account {}", Uuid::new_v4()))
        .email(format!("revoked_{}@example.com", Uuid::new_v4()))
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
        .name("Revoked Test Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(10)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_rate_limit_per_minute()
        .expect_created_at()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created with status: active");

    // Revoke the API key
    let revoked_key = ApiKeyBuilder::new()
        .id(api_key.id)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_created_at()
        .expect_revoked_at()
        .revoke(Some(&mut conn))
        .await
        .expect("Failed to revoke API key");

    println!("🚫 API key revoked");

    // Try to use revoked API key
    println!("\n🔍 Attempting to use revoked API key...");
    let result = RateLimiter::check_rate_limit(revoked_key.id, &key_prefix).await;

    assert!(result.is_err(), "Revoked API key should fail");
    match result {
        Err(ServiceError::ValidationError(msg)) => {
            println!("✅ Correctly rejected: {}", msg);
            assert!(msg.contains("revoked"));
        }
        _ => panic!("Expected ValidationError for revoked key"),
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
async fn test_rate_limit_with_backoff_retry() {
    println!("\n=== TEST: Rate Limit with Exponential Backoff Retry ===");

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
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE TRUE")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Backoff Test%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Backoff Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("Backoff Test Account {}", Uuid::new_v4()))
        .email(format!("backoff_{}@example.com", Uuid::new_v4()))
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

    // Create API key with very low rate limit (2 per minute) to test retry
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Backoff Test Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(2)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_rate_limit_per_minute()
        .expect_rate_limit_per_hour()
        .expect_created_at()
        .expect_last_used_at()
        .expect_expires_at()
        .expect_permissions()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created with rate_limit_per_minute = 2");

    // Make 2 requests to exhaust the limit
    println!("\n📊 Making 2 requests to exhaust limit...");
    for i in 1..=2 {
        let result = RateLimiter::check_rate_limit(api_key.id, &key_prefix).await;
        assert!(result.is_ok(), "Request {} should succeed", i);
        println!("   Request {}: succeeded", i);
    }

    // Now the limit is exhausted - test retry with backoff
    println!("\n🔄 Testing retry with exponential backoff...");
    println!("   This will retry with backoff and should eventually succeed");

    // Use check_with_retry with very short delays for testing
    // max_retries: 2, base_delay: 50ms, max_delay: 200ms
    let start_time = std::time::Instant::now();
    let result = RateLimiter::check_with_retry(
        api_key.id,
        &key_prefix,
        2,   // 2 retries
        50,  // 50ms base delay
        2000, // 2000ms max delay
    )
    .await;
    let elapsed = start_time.elapsed();

    // The retry should fail after exhausting retries (within the same minute)
    // because the rate limit window hasn't reset yet
    match result {
        Ok(_) => {
            println!("✅ Request succeeded after retry (window may have reset)");
            println!("   Elapsed time: {:?}", elapsed);
        }
        Err(ServiceError::RateLimitExceeded {
            limit,
            window,
            reset_at,
        }) => {
            println!("✅ Rate limit still exceeded after retries (as expected):");
            println!("   - Limit: {}", limit);
            println!("   - Window: {}", window);
            println!("   - Reset at: {}", reset_at);
            println!("   - Elapsed time: {:?}", elapsed);

            // Verify we actually retried (should take at least base_delay * attempts)
            assert!(
                elapsed.as_millis() >= 50,
                "Should have taken time for retries"
            );

            assert_eq!(limit, 2);
            assert_eq!(window, "minute");
        }
        Err(e) => panic!("Unexpected error: {:?}", e),
    }

    // Test that immediate check still fails
    println!("\n🚫 Verifying rate limit is still active...");
    let immediate_result = RateLimiter::check_rate_limit(api_key.id, &key_prefix).await;
    assert!(immediate_result.is_err(), "Should still be rate limited");
    println!("✅ Rate limit still active as expected");

    // Cleanup
    let _ = sqlx::query("DELETE FROM rate_limit_counters WHERE api_key_id = $1")
        .bind(api_key.id)
        .execute(&mut *conn)
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
