use mockall::predicate::str::contains;
use payments_backend_dodo::datalayer::CRUD::accounts::AccountBuilder;
use payments_backend_dodo::datalayer::db_ops::db_ops::initialize_database;
use payments_backend_dodo::errors::errors::ServiceError;

// Load environment variables from .env file
#[tokio::test]
async fn test_account_create_update_read_flow() {
    println!("\n=== TEST: Account Create-Update-Read Flow ===");

    // Load .env file
    let _ = dotenvy::dotenv();

    // Skip test if DATABASE_URL is not set
    if std::env::var("DATABASE_URL").is_err() {
        println!("⚠️  Skipping test: DATABASE_URL not set");
        return;
    }

    // Initialize database and get DbOps
    println!("🔧 Initializing database...");
    let db_ops = match initialize_database().await {
        Ok(ops) => ops,
        Err(e) => {
            println!("❌ Failed to initialize database: {}", e);
            return;
        }
    };

    println!("✅ Database initialized successfully");

    // Get a connection from the pool
    let mut conn = match db_ops.tracker().get_connection().await {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to get connection: {}", e);
            db_ops.shutdown().await;
            return;
        }
    };

    println!("✅ Connection acquired from pool");

    // === STEP 1: CREATE ===
    println!("\n📝 STEP 1: Creating a new account...");

    let create_result = AccountBuilder::new()
        .business_name("Test Business Inc.".to_string())
        .email("test@example.com".to_string())
        .currency("USD".to_string())
        .status("active".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_currency()
        .expect_balance()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .create(Some(&mut conn))
        .await;

    let created_account = match create_result {
        Ok(account) => {
            println!("✅ Account created successfully");
            println!("   - ID: {}", account.id);
            println!("   - Business Name: {}", account.business_name);
            println!("   - Email: {}", account.email);
            println!("   - Balance: {}", account.balance);
            println!("   - Currency: {}", account.currency);
            println!("   - Status: {}", account.status);
            println!("   - Created At: {}", account.created_at);
             
             // Assertions
            assert_eq!(account.business_name, "Test Business Inc.");
            assert_eq!(account.email, "test@example.com");
            assert_eq!(account.balance, 0.0);
            assert_eq!(account.currency, "USD");
            assert_eq!(account.status, "active");

            account
        }
        Err(e) => {
            println!("❌ Failed to create account: {:?}", e);
            db_ops.tracker().return_connection(conn);    
            // dont return here just contineu
            return;
        }
    };

    // === STEP 2: UPDATE ===
    println!("\n🔄 STEP 2: Updating the account...");

    let update_result = AccountBuilder::new()
        .id(created_account.id)
        .business_name("Updated Business Name".to_string())
        .status("suspended".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_currency()
        .expect_balance()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .update(Some(&mut conn))
        .await;

    let updated_account = match update_result {
        Ok(account) => {
            println!("✅ Account updated successfully");
            println!("   - ID: {}", account.id);
            println!("   - Business Name: {}", account.business_name);
            println!("   - Status: {}", account.status);
            println!("   - Updated At: {}", account.updated_at);

            // Assertions
            assert_eq!(account.id, created_account.id);
            assert_eq!(account.business_name, "Updated Business Name");
            assert_eq!(account.status, "suspended");
            assert_eq!(account.email, created_account.email); // Email should remain unchanged
            assert!(account.updated_at > created_account.updated_at); // Updated timestamp should be newer

            account
        }
        Err(e) => {
            println!("❌ Failed to update account: {:?}", e);
            db_ops.tracker().return_connection(conn);
            db_ops.shutdown().await;
            panic!("Account update failed");
        }
    };

    // === STEP 3: READ ===
    println!("\n📖 STEP 3: Reading the account by ID...");

    let read_result = AccountBuilder::new()
        .id(created_account.id)
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_currency()
        .expect_balance()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .read(Some(&mut conn))
        .await;

    match read_result {
        Ok(account) => {
            println!("✅ Account read successfully");
            println!("   - ID: {}", account.id);
            println!("   - Business Name: {}", account.business_name);
            println!("   - Email: {}", account.email);
            println!("   - Status: {}", account.status);

            // Assertions
            assert_eq!(account.id, updated_account.id);
            assert_eq!(account.business_name, updated_account.business_name);
            assert_eq!(account.email, updated_account.email);
            assert_eq!(account.status, updated_account.status);
            assert_eq!(account.balance, updated_account.balance);
        }
        Err(e) => {
            println!("❌ Failed to read account: {:?}", e);
            db_ops.tracker().return_connection(conn);
            db_ops.shutdown().await;
            panic!("Account read failed");
        }
    };

    // === STEP 3.5: UPDATE BALANCE ===
    println!("\n🔄 STEP 3.5: Updating the account balance...");
     let update_result = AccountBuilder::new()
        .id(created_account.id)
        .balance(100.0)
        .currency("kwd".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_currency()
        .expect_balance()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .update(None)
        .await;

    match update_result {
        Ok(account) => {
            println!("✅ Account updated successfully");
            println!("   - ID: {}", account.id);
            println!("   - Business Name: {}", account.business_name);
            println!("   - Email: {}", account.email);
            println!("   - Status: {}", account.status);
            println!("   - Updated At: {}", account.updated_at);
            println!("   - Balance: {}", account.balance);

            // Assertions
            assert_eq!(account.id, created_account.id);
            assert_eq!(account.email, updated_account.email);
            assert_eq!(account.status, updated_account.status);
        }
        Err(e) => {
            println!("❌ Failed to update account: {:?}", e);
            db_ops.tracker().return_connection(conn);
            db_ops.shutdown().await;
            panic!("Account update failed");
        }
    };


    // === STEP 4: READ BY EMAIL ===
    println!("\n📖 STEP 4: Reading the account by email...");

    let read_by_email_result = AccountBuilder::new()
        .email("test@example.com".to_string())
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_currency()
        .expect_balance()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .read(Some(&mut conn))
        .await;

    match read_by_email_result {
        Ok(account) => {
            println!("✅ Account read by email successfully");
            println!("   - ID: {}", account.id);
            println!("   - Email: {}", account.email);

            // Assertions
            assert_eq!(account.id, created_account.id);
            assert_eq!(account.email, "test@example.com");
        }
        Err(e) => {
            println!("❌ Failed to read account by email: {:?}", e);
            db_ops.tracker().return_connection(conn);
            db_ops.shutdown().await;
            panic!("Account read by email failed");
        }
    };

    // Return connection to pool
    db_ops.tracker().return_connection(conn);
    println!("✅ Connection returned to pool");

    // Shutdown database
    db_ops.shutdown().await;
    println!("✅ Database shut down");

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
}

#[tokio::test]
async fn test_account_duplicate_email() {
    println!("\n=== TEST: Account Duplicate Email ===");

    // Load .env file
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

    // Create first account
    println!("\n📝 Creating first account...");
    let first_account = AccountBuilder::new()
        .business_name("First Business".to_string())
        .email("duplicate@example.com".to_string())
        .expect_id()
        .create(Some(&mut conn))
        .await;

    let created_id = match first_account {
        Ok(account) => {
            println!("✅ First account created: {}", account.id);
            account.id
        }
        Err(e) => {
            println!("❌ Failed to create first account: {:?}", e);
            db_ops.tracker().return_connection(conn);
            db_ops.shutdown().await;
            panic!("First account creation failed");
        }
    };

    // Try to create second account with same email
    println!("\n📝 Attempting to create account with duplicate email...");
    let duplicate_result = AccountBuilder::new()
        .business_name("Second Business".to_string())
        .email("duplicate@example.com".to_string())
        .create(Some(&mut conn))
        .await;

    match duplicate_result {
        Ok(_) => {
            println!("❌ Duplicate account was created (should have failed!)");

            // Cleanup
            let _ = sqlx::query("DELETE FROM accounts WHERE id = $1")
                .bind(created_id)
                .execute(&mut *conn)
                .await;

            db_ops.tracker().return_connection(conn);
            db_ops.shutdown().await;
            panic!("Duplicate email should have been rejected");
        }
        Err(ServiceError::AccountAlreadyExists(_)) => {
            println!("✅ Duplicate email correctly rejected");
        }
        Err(e) => {
            println!("⚠️  Unexpected error: {:?}", e);
        }
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(created_id)
        .execute(&mut *conn)
        .await;

    db_ops.tracker().return_connection(conn);
    db_ops.shutdown().await;

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
}

#[tokio::test]
async fn test_account_check_exists() {
    println!("\n=== TEST: Account Check Exists ===");

    // Load .env file
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

    // Create an account
    println!("\n📝 Creating test account...");
    let account = AccountBuilder::new()
        .business_name("Exists Test Business".to_string())
        .email("exists@example.com".to_string())
        .expect_id()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create test account");

    println!("✅ Test account created: {}", account.id);

    // Test check_exists with matching email
    println!("\n🔍 Checking if account exists by email...");
    let exists_by_email =
        AccountBuilder::check_exists("Different Business", "exists@example.com", &mut *conn)
            .await
            .expect("check_exists failed");

    assert!(exists_by_email, "Account should exist by email");
    println!("✅ Account found by email");

    // Test check_exists with matching business name
    println!("\n🔍 Checking if account exists by business name...");
    let exists_by_name =
        AccountBuilder::check_exists("Exists Test Business", "different@example.com", &mut *conn)
            .await
            .expect("check_exists failed");

    assert!(exists_by_name, "Account should exist by business name");
    println!("✅ Account found by business name");

    // Test check_exists with non-matching values
    println!("\n🔍 Checking if non-existent account exists...");
    let not_exists = AccountBuilder::check_exists(
        "Non Existent Business",
        "nonexistent@example.com",
        &mut *conn,
    )
    .await
    .expect("check_exists failed");

    assert!(!not_exists, "Account should not exist");
    println!("✅ Non-existent account correctly not found");

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM accounts WHERE id = $1")
        .bind(account.id)
        .execute(&mut *conn)
        .await;

    db_ops.tracker().return_connection(conn);
    db_ops.shutdown().await;

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
}
