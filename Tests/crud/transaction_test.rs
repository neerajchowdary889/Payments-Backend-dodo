use payments_backend_dodo::datalayer::CRUD::accounts::AccountBuilder;
use payments_backend_dodo::datalayer::CRUD::transaction::TransactionBuilder;
use payments_backend_dodo::datalayer::CRUD::types::{TransactionStatus, TransactionType};
use payments_backend_dodo::datalayer::db_ops::db_ops::initialize_database;
use payments_backend_dodo::errors::errors::ServiceError;
use uuid::Uuid;

/// Test complete create and read flow for a Credit transaction
#[tokio::test]
async fn test_transaction_create_read_flow() {
    println!("\n=== TEST: Transaction Create-Read Flow ===");

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

    println!("✅ Database initialized successfully");

    let mut conn = match db_ops.tracker().get_connection().await {
        Ok(c) => c,
        Err(e) => {
            println!("❌ Failed to get connection: {}", e);
            db_ops.shutdown().await;
            return;
        }
    };

    println!("✅ Connection acquired from pool");

    // === STEP 1: CREATE TEST ACCOUNT (without balance) ===
    println!("\n📝 STEP 1: Creating test account without balance...");

    let account = AccountBuilder::new()
        .business_name(format!("Transaction Test Account {}", Uuid::new_v4()))
        .email(format!("txn_test_{}@example.com", Uuid::new_v4()))
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
        .expect("Failed to create test account");

    println!("✅ Account created with ID: {}", account.id);
    println!("   - Initial Balance: ${}", account.balance);

    // === STEP 2: UPDATE ACCOUNT WITH BALANCE ===
    println!("\n💰 STEP 2: Updating account with balance...");

    let updated_account = AccountBuilder::new()
        .id(account.id)
        .balance(1000.0)
        .expect_id()
        .expect_business_name()
        .expect_email()
        .expect_balance()
        .expect_currency()
        .expect_status()
        .expect_created_at()
        .expect_updated_at()
        .update(Some(&mut conn))
        .await
        .expect("Failed to update account balance");

    println!("✅ Account balance updated");
    println!("   - New Balance: ${}", updated_account.balance);
    assert_eq!(updated_account.balance, 1000.0);

    // === STEP 3: CREATE TRANSACTION ===
    println!("\n📝 STEP 3: Creating Credit transaction...");

    let idempotency_key = format!("test_txn_{}", Uuid::new_v4());
    // For standalone transactions, use idempotency_key as parent_tx_key
    // to avoid validation that looks for parent transactions
    let parent_tx_key = idempotency_key.clone();

    let created_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account.id)
        .amount(250.0)
        .currency("USD".to_string())
        .idempotency_key(idempotency_key.clone())
        .parent_tx_key(parent_tx_key.clone())
        .description("Test credit transaction".to_string())
        .expect_id()
        .expect_transaction_type()
        .expect_amount()
        .expect_currency()
        .expect_status()
        .expect_idempotency_key()
        .expect_parent_tx_key()
        .expect_created_at()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create transaction");

    println!("✅ Transaction created successfully");
    println!("   - ID: {}", created_txn.id);
    println!("   - Type: {:?}", created_txn.transaction_type);
    println!("   - Amount: ${}", created_txn.amount);
    println!("   - Status: {:?}", created_txn.status);
    println!("   - Idempotency Key: {}", created_txn.idempotency_key);

    // Assertions
    assert_eq!(created_txn.transaction_type, TransactionType::Credit);
    assert_eq!(created_txn.amount, 250.0);
    assert_eq!(created_txn.currency, "USD");
    assert_eq!(created_txn.status, TransactionStatus::Completed);
    assert_eq!(created_txn.idempotency_key, idempotency_key);
    assert_eq!(created_txn.parent_tx_key, parent_tx_key);

    // === STEP 4: READ TRANSACTION BY ID ===
    println!("\n📖 STEP 4: Reading transaction by ID...");

    let read_txn = TransactionBuilder::new()
        .id(created_txn.id)
        .expect_id()
        .expect_transaction_type()
        .expect_amount()
        .expect_currency()
        .expect_status()
        .expect_idempotency_key()
        .expect_parent_tx_key()
        .expect_created_at()
        .read(Some(&mut conn))
        .await
        .expect("Failed to read transaction");

    println!("✅ Transaction read successfully");
    println!("   - ID: {}", read_txn.id);
    println!("   - Amount: ${}", read_txn.amount);

    // Assertions
    assert_eq!(read_txn.id, created_txn.id);
    assert_eq!(read_txn.transaction_type, created_txn.transaction_type);
    assert_eq!(read_txn.amount, created_txn.amount);
    assert_eq!(read_txn.status, created_txn.status);
    assert_eq!(read_txn.idempotency_key, created_txn.idempotency_key);

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM transactions WHERE id = $1")
        .bind(created_txn.id)
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

/// Test updating transaction status and error fields
#[tokio::test]
async fn test_transaction_update_flow() {
    println!("\n=== TEST: Transaction Update Flow ===");

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

    // Create test account
    println!("\n📝 Creating test account...");
    let account = AccountBuilder::new()
        .business_name("Update Test Account".to_string())
        .email(format!("update_test_{}@example.com", Uuid::new_v4()))
        .balance(500.0)
        .expect_id()
        .expect_balance()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    println!("✅ Account created: {}", account.id);

    // Create transaction
    println!("\n📝 Creating Debit transaction...");
    let created_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Debit)
        .from_account_id(account.id)
        .amount(100.0)
        .idempotency_key(format!("update_test_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .expect_id()
        .expect_status()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create transaction");

    println!(
        "✅ Transaction created with status: {:?}",
        created_txn.status
    );

    // Update transaction to Failed status
    println!("\n🔄 Updating transaction to Failed status...");
    let updated_txn = TransactionBuilder::new()
        .id(created_txn.id)
        .status(TransactionStatus::Failed)
        .error_code("INSUFFICIENT_FUNDS".to_string())
        .error_message("Account has insufficient funds".to_string())
        .expect_id()
        .expect_status()
        .expect_error_code()
        .expect_error_message()
        .update(Some(&mut conn))
        .await
        .expect("Failed to update transaction");

    println!("✅ Transaction updated successfully");
    println!("   - Status: {:?}", updated_txn.status);
    println!("   - Error Code: {:?}", updated_txn.error_code);

    // Assertions
    assert_eq!(updated_txn.id, created_txn.id);
    assert_eq!(updated_txn.status, TransactionStatus::Failed);
    assert_eq!(
        updated_txn.error_code,
        Some("INSUFFICIENT_FUNDS".to_string())
    );
    assert_eq!(
        updated_txn.error_message,
        Some("Account has insufficient funds".to_string())
    );

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM transactions WHERE id = $1")
        .bind(created_txn.id)
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

/// Test idempotency key prevents duplicate transactions
#[tokio::test]
async fn test_transaction_duplicate_idempotency_key() {
    println!("\n=== TEST: Duplicate Idempotency Key ===");

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

    // Create test account
    println!("\n📝 Creating test account...");
    let account = AccountBuilder::new()
        .business_name("Idempotency Test Account".to_string())
        .email(format!("idem_test_{}@example.com", Uuid::new_v4()))
        .expect_id()
        .expect_balance()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    println!("✅ Account created: {}", account.id);

    // Create first transaction
    let idempotency_key = format!("duplicate_test_{}", Uuid::new_v4());
    let parent_tx_key = format!("parent_{}", Uuid::new_v4());

    println!("\n📝 Creating first transaction...");
    let first_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account.id)
        .amount(100.0)
        .idempotency_key(idempotency_key.clone())
        .parent_tx_key(parent_tx_key.clone())
        .expect_id()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create first transaction");

    println!("✅ First transaction created: {}", first_txn.id);

    // Attempt to create duplicate transaction
    println!("\n📝 Attempting to create duplicate transaction...");
    let duplicate_result = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account.id)
        .amount(200.0) // Different amount
        .idempotency_key(idempotency_key.clone()) // Same idempotency key
        .parent_tx_key(parent_tx_key.clone())
        .create(Some(&mut conn))
        .await;

    match duplicate_result {
        Ok(_) => {
            println!("❌ Duplicate transaction was created (should have failed!)");
            panic!("Duplicate idempotency key should have been rejected");
        }
        Err(ServiceError::DuplicateTransaction(_)) => {
            println!("✅ Duplicate idempotency key correctly rejected");
        }
        Err(e) => {
            println!("⚠️  Unexpected error: {:?}", e);
            panic!("Expected DuplicateTransaction error");
        }
    }

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM transactions WHERE id = $1")
        .bind(first_txn.id)
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

/// Test reading transaction by idempotency key
#[tokio::test]
async fn test_transaction_read_by_idempotency_key() {
    println!("\n=== TEST: Read by Idempotency Key ===");

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

    // Create test account
    println!("\n📝 Creating test account...");
    let account = AccountBuilder::new()
        .business_name("Read Test Account".to_string())
        .email(format!("read_test_{}@example.com", Uuid::new_v4()))
        .expect_id()
        .expect_balance()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    // Create transaction
    let idempotency_key = format!("read_test_{}", Uuid::new_v4());
    println!(
        "\n📝 Creating transaction with idempotency key: {}",
        idempotency_key
    );

    let created_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account.id)
        .amount(150.0)
        .idempotency_key(idempotency_key.clone())
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .expect_id()
        .expect_amount()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create transaction");

    println!("✅ Transaction created: {}", created_txn.id);

    // Read by idempotency key
    println!("\n📖 Reading transaction by idempotency key...");
    let read_txn = TransactionBuilder::new()
        .idempotency_key(idempotency_key.clone())
        .expect_id()
        .expect_amount()
        .expect_idempotency_key()
        .read(Some(&mut conn))
        .await
        .expect("Failed to read transaction by idempotency key");

    println!("✅ Transaction read successfully");
    println!("   - ID: {}", read_txn.id);
    println!("   - Amount: ${}", read_txn.amount);

    // Assertions
    assert_eq!(read_txn.id, created_txn.id);
    assert_eq!(read_txn.amount, created_txn.amount);
    assert_eq!(read_txn.idempotency_key, idempotency_key);

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM transactions WHERE id = $1")
        .bind(created_txn.id)
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

/// Test all three transaction types (Credit, Debit, Transfer)
#[tokio::test]
async fn test_transaction_types() {
    println!("\n=== TEST: All Transaction Types ===");

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

    // Create two test accounts
    println!("\n📝 Creating test accounts...");
    let account1 = AccountBuilder::new()
        .business_name("Account 1".to_string())
        .email(format!("acc1_{}@example.com", Uuid::new_v4()))
        .balance(1000.0)
        .expect_id()
        .expect_balance()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account 1");

    let account2 = AccountBuilder::new()
        .business_name("Account 2".to_string())
        .email(format!("acc2_{}@example.com", Uuid::new_v4()))
        .balance(500.0)
        .expect_id()
        .expect_balance()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account 2");

    println!("✅ Accounts created: {} and {}", account1.id, account2.id);

    // Test Credit transaction
    println!("\n💰 Testing Credit transaction...");
    let credit_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account1.id)
        .amount(100.0)
        .idempotency_key(format!("credit_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .expect_id()
        .expect_transaction_type()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create credit transaction");

    assert_eq!(credit_txn.transaction_type, TransactionType::Credit);
    assert_eq!(credit_txn.to_account_id, Some(account1.id));
    assert_eq!(credit_txn.from_account_id, None);
    println!("✅ Credit transaction created: {}", credit_txn.id);

    // Test Debit transaction
    println!("\n💸 Testing Debit transaction...");
    let debit_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Debit)
        .from_account_id(account1.id)
        .amount(50.0)
        .idempotency_key(format!("debit_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .expect_id()
        .expect_transaction_type()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create debit transaction");

    assert_eq!(debit_txn.transaction_type, TransactionType::Debit);
    assert_eq!(debit_txn.from_account_id, Some(account1.id));
    assert_eq!(debit_txn.to_account_id, None);
    println!("✅ Debit transaction created: {}", debit_txn.id);

    // Test Transfer transaction
    println!("\n🔄 Testing Transfer transaction...");
    let transfer_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Transfer)
        .from_account_id(account1.id)
        .to_account_id(account2.id)
        .amount(75.0)
        .idempotency_key(format!("transfer_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .expect_id()
        .expect_transaction_type()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create transfer transaction");

    assert_eq!(transfer_txn.transaction_type, TransactionType::Transfer);
    assert_eq!(transfer_txn.from_account_id, Some(account1.id));
    assert_eq!(transfer_txn.to_account_id, Some(account2.id));
    println!("✅ Transfer transaction created: {}", transfer_txn.id);

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM transactions WHERE id IN ($1, $2, $3)")
        .bind(credit_txn.id)
        .bind(debit_txn.id)
        .bind(transfer_txn.id)
        .execute(&mut *conn)
        .await;
    let _ = sqlx::query("DELETE FROM accounts WHERE id IN ($1, $2)")
        .bind(account1.id)
        .bind(account2.id)
        .execute(&mut *conn)
        .await;

    db_ops.tracker().return_connection(conn);
    db_ops.shutdown().await;

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
}

/// Test transaction validation rules
#[tokio::test]
async fn test_transaction_validation() {
    println!("\n=== TEST: Transaction Validation ===");

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

    // Create test account
    let account = AccountBuilder::new()
        .business_name("Validation Test".to_string())
        .email(format!("validation_{}@example.com", Uuid::new_v4()))
        .expect_id()
        .expect_balance()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    // Test 1: Missing transaction_type
    println!("\n🔍 Test 1: Missing transaction_type...");
    let result = TransactionBuilder::new()
        .to_account_id(account.id)
        .amount(100.0)
        .idempotency_key(format!("test_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .create(Some(&mut conn))
        .await;

    assert!(result.is_err());
    println!("✅ Missing transaction_type correctly rejected");

    // Test 2: Missing amount
    println!("\n🔍 Test 2: Missing amount...");
    let result = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account.id)
        .idempotency_key(format!("test_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .create(Some(&mut conn))
        .await;

    assert!(result.is_err());
    println!("✅ Missing amount correctly rejected");

    // Test 3: Credit with from_account_id (should fail)
    println!("\n🔍 Test 3: Credit with from_account_id...");
    let result = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .from_account_id(account.id) // Should not have from_account_id
        .to_account_id(account.id)
        .amount(100.0)
        .idempotency_key(format!("test_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .create(Some(&mut conn))
        .await;

    assert!(result.is_err());
    println!("✅ Credit with from_account_id correctly rejected");

    // Test 4: Debit without from_account_id (should fail)
    println!("\n🔍 Test 4: Debit without from_account_id...");
    let result = TransactionBuilder::new()
        .transaction_type(TransactionType::Debit)
        .amount(100.0)
        .idempotency_key(format!("test_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .create(Some(&mut conn))
        .await;

    assert!(result.is_err());
    println!("✅ Debit without from_account_id correctly rejected");

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

/// Test connection management (with and without provided connection)
#[tokio::test]
async fn test_transaction_connection_management() {
    println!("\n=== TEST: Connection Management ===");

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

    // Create test account
    let account = AccountBuilder::new()
        .business_name("Connection Test".to_string())
        .email(format!("conn_test_{}@example.com", Uuid::new_v4()))
        .expect_id()
        .expect_balance()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create account");

    // Test 1: Create with provided connection
    println!("\n🔌 Test 1: Creating transaction with provided connection...");
    let txn1 = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account.id)
        .amount(100.0)
        .idempotency_key(format!("conn_test_1_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .expect_id()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create transaction with provided connection");

    println!(
        "✅ Transaction created with provided connection: {}",
        txn1.id
    );

    // Test 2: Create without connection (auto-acquire)
    println!("\n🔌 Test 2: Creating transaction without connection (auto-acquire)...");
    let txn2 = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account.id)
        .amount(150.0)
        .idempotency_key(format!("conn_test_2_{}", Uuid::new_v4()))
        .parent_tx_key(format!("parent_{}", Uuid::new_v4()))
        .expect_id()
        .create(None) // No connection provided
        .await
        .expect("Failed to create transaction with auto-acquired connection");

    println!(
        "✅ Transaction created with auto-acquired connection: {}",
        txn2.id
    );

    // Test 3: Read without connection
    println!("\n🔌 Test 3: Reading transaction without connection...");
    let read_txn = TransactionBuilder::new()
        .id(txn1.id)
        .expect_id()
        .read(None) // No connection provided
        .await
        .expect("Failed to read transaction without connection");

    assert_eq!(read_txn.id, txn1.id);
    println!("✅ Transaction read with auto-acquired connection");

    // Cleanup
    println!("\n🧹 Cleaning up...");
    let _ = sqlx::query("DELETE FROM transactions WHERE id IN ($1, $2)")
        .bind(txn1.id)
        .bind(txn2.id)
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
