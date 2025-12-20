use payments_backend_dodo::datalayer::CRUD::accounts::AccountBuilder;
use payments_backend_dodo::datalayer::CRUD::api_key::ApiKeyBuilder;
use payments_backend_dodo::datalayer::db_ops::db_ops::initialize_database;
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
async fn test_create_api_key() {
    println!("\n=== TEST: Create API Key ===");

    let _ = dotenvy::dotenv();

    if std::env::var("DATABASE_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL not set");
        return;
    }

    println!("🔧 Initializing database...");
    let db_ops = match initialize_database().await {
        Ok(ops) => ops,
        Err(e) => {
            println!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    let mut conn = match db_ops.tracker().get_connection().await {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to get connection: {}", e);
            db_ops.shutdown().await;
            return;
        }
    };

    println!("✅ Database initialized and connection acquired");

    // === CLEANUP: Remove any leftover test data ===
    println!("\n🧹 Cleaning up any leftover test data...");
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Test API Key%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%API Key Test Account%'")
        .execute(&mut *conn)
        .await;

    // === STEP 1: CREATE ACCOUNT ===
    println!("\n📝 STEP 1: Creating test account...");
    let account = AccountBuilder::new()
        .business_name(format!("API Key Test Account {}", Uuid::new_v4()))
        .email(format!("apikey_test_{}@example.com", Uuid::new_v4()))
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

    println!("✅ Account created: {}", account.id);

    // === STEP 2: CREATE API KEY ===
    println!("\n🔑 STEP 2: Creating API key...");
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);

    let api_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Test API Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(100)
        .rate_limit_per_hour(1000)
        .permissions(serde_json::json!(["read", "write"]))
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

    println!("✅ API key created:");
    println!("   - ID: {}", api_key.id);
    println!("   - Account ID: {}", api_key.account_id);
    println!("   - Key Prefix: {}", api_key.key_prefix);
    println!("   - Name: {:?}", api_key.name);
    println!("   - Status: {}", api_key.status);


    // === VERIFICATION ===
    assert_eq!(api_key.account_id, account.id);
    assert_eq!(api_key.key_hash, key_hash);
    assert_eq!(api_key.key_prefix, key_prefix);
    assert_eq!(api_key.name, Some("Test API Key".to_string()));
    assert_eq!(api_key.status, "active");

    // Cleanup
    println!("\n🧹 Cleaning up...");
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
async fn test_read_api_key() {
    println!("\n=== TEST: Read API Key ===");

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
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Read Test API Key%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%API Key Read Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("API Key Read Test {}", Uuid::new_v4()))
        .email(format!("apikey_read_{}@example.com", Uuid::new_v4()))
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

    let created_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Read Test API Key".to_string())
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

    println!("✅ API key created with ID: {}", created_key.id);

    // === TEST READ BY ID ===
    println!("\n🔍 Reading API key by ID...");
    let read_key = ApiKeyBuilder::new()
        .id(created_key.id)
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
        .read(Some(&mut conn))
        .await
        .expect("Failed to read API key");

    println!("✅ API key read successfully:");
    println!("   - ID: {}", read_key.id);
    println!("   - Account ID: {}", read_key.account_id);
    println!("   - Key Prefix: {}", read_key.key_prefix);

    assert_eq!(read_key.id, created_key.id);
    assert_eq!(read_key.account_id, created_key.account_id);
    assert_eq!(read_key.key_hash, created_key.key_hash);

    // === TEST READ BY KEY PREFIX ===
    println!("\n🔍 Reading API key by key_prefix...");
    let read_by_prefix = ApiKeyBuilder::new()
        .key_prefix(key_prefix.clone())
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_created_at()
        .read(Some(&mut conn))
        .await
        .expect("Failed to read API key by prefix");

    assert_eq!(read_by_prefix.id, created_key.id);
    println!("✅ API key found by prefix");

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(created_key.id)
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
async fn test_update_api_key() {
    println!("\n=== TEST: Update API Key ===");

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
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Update Test API Key%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%API Key Update Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("API Key Update Test {}", Uuid::new_v4()))
        .email(format!("apikey_update_{}@example.com", Uuid::new_v4()))
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

    let created_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Update Test API Key".to_string())
        .status("active".to_string())
        .rate_limit_per_minute(50)
        .rate_limit_per_hour(500)
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

    println!("✅ API key created with rate limits: 50/min, 500/hour");

    // === UPDATE API KEY ===
    println!("\n🔄 Updating API key...");
    let updated_key = ApiKeyBuilder::new()
        .id(created_key.id)
        .name("Updated API Key Name".to_string())
        .rate_limit_per_minute(200)
        .rate_limit_per_hour(2000)
        .permissions(serde_json::json!(["read"]))
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
        .update(Some(&mut conn))
        .await
        .expect("Failed to update API key");

    println!("✅ API key updated:");
    println!("   - New Name: {:?}", updated_key.name);

    assert_eq!(updated_key.id, created_key.id);
    assert_eq!(updated_key.name, Some("Updated API Key Name".to_string()));

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(created_key.id)
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
async fn test_revoke_api_key() {
    println!("\n=== TEST: Revoke API Key ===");

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
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Revoke Test API Key%'")
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%API Key Revoke Test%'")
        .execute(&mut *conn)
        .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("API Key Revoke Test {}", Uuid::new_v4()))
        .email(format!("apikey_revoke_{}@example.com", Uuid::new_v4()))
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

    let created_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Revoke Test API Key".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_created_at()
        .expect_revoked_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created with status: {}", created_key.status);
    assert_eq!(created_key.status, "active");
    assert!(created_key.revoked_at.is_none());

    // === REVOKE API KEY ===
    println!("\n🚫 Revoking API key...");
    let revoked_key = ApiKeyBuilder::new()
        .id(created_key.id)
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

    println!("✅ API key revoked:");
    println!("   - Status: {}", revoked_key.status);
    println!("   - Revoked At: {:?}", revoked_key.revoked_at);

    assert_eq!(revoked_key.id, created_key.id);
    assert_eq!(revoked_key.status, "revoked");
    assert!(revoked_key.revoked_at.is_some());

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(created_key.id)
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
async fn test_api_key_with_expiration() {
    println!("\n=== TEST: API Key with Expiration ===");

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
    let _ = sqlx::query("DELETE FROM api_keys WHERE name LIKE '%Expiration Test API Key%'")
        .execute(&mut *conn)
        .await;
    let _ =
        sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%API Key Expiration Test%'")
            .execute(&mut *conn)
            .await;

    // Create account
    let account = AccountBuilder::new()
        .business_name(format!("API Key Expiration Test {}", Uuid::new_v4()))
        .email(format!("apikey_exp_{}@example.com", Uuid::new_v4()))
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

    // Create API key with expiration
    let api_key_value = format!("sk_test_{}", Uuid::new_v4());
    let key_hash = hash_api_key(&api_key_value);
    let key_prefix = get_key_prefix(&api_key_value);
    let expires_at = chrono::Utc::now() + chrono::Duration::days(30);

    let created_key = ApiKeyBuilder::new()
        .account_id(account.id)
        .key_hash(key_hash.clone())
        .key_prefix(key_prefix.clone())
        .name("Expiration Test API Key".to_string())
        .status("active".to_string())
        .expires_at(expires_at)
        .expect_id()
        .expect_account_id()
        .expect_key_hash()
        .expect_key_prefix()
        .expect_name()
        .expect_status()
        .expect_expires_at()
        .expect_created_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create API key");

    println!("✅ API key created with expiration:");
    println!("   - Expires At: {:?}", created_key.expires_at);

    assert!(created_key.expires_at.is_some());
    let expires = created_key.expires_at.unwrap();
    assert!(expires > chrono::Utc::now());

    // Cleanup
    let _ = sqlx::query("DELETE FROM api_keys WHERE id = $1")
        .bind(created_key.id)
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
