use payments_backend_dodo::datalayer::CRUD::accounts::AccountBuilder;
use payments_backend_dodo::datalayer::CRUD::transaction::TransactionBuilder;
use payments_backend_dodo::datalayer::CRUD::types::{TransactionStatus, TransactionType};
use payments_backend_dodo::datalayer::db_ops::db_ops::initialize_database;
use uuid::Uuid;

/// Test the complete transfer flow: Transfer -> Debit -> Credit with linked parent_tx_key
#[tokio::test]
async fn test_transfer_debit_credit_flow() {
    println!("\n=== TEST: Transfer -> Debit -> Credit Flow ===");

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

    // === CLEANUP: Remove any leftover test data from previous failed runs ===
    println!("\n🧹 Cleaning up any leftover test data...");

    // Delete transactions first (due to foreign key constraints)
    let deleted_txns = sqlx::query("DELETE FROM transactions WHERE description LIKE '%transfer%' OR idempotency_key LIKE 'idem_%'")
        .execute(&mut *conn)
        .await
        .map(|r| r.rows_affected())
        .unwrap_or(0);

    // Then delete accounts
    let deleted_accounts =
        sqlx::query("DELETE FROM accounts WHERE business_name LIKE '%Transfer Test Account%'")
            .execute(&mut *conn)
            .await
            .map(|r| r.rows_affected())
            .unwrap_or(0);

    println!(
        "✅ Cleanup complete: {} transactions, {} accounts deleted",
        deleted_txns, deleted_accounts
    );

    // === STEP 1: CREATE TWO ACCOUNTS ===
    println!("\n📝 STEP 1: Creating two accounts without balance...");

    let account1 = AccountBuilder::new()
        .business_name(format!("Transfer Test Account 1 {}", Uuid::new_v4()))
        .email(format!("transfer1_{}@example.com", Uuid::new_v4()))
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
        .expect("Failed to create account 1");

    let account2 = AccountBuilder::new()
        .business_name(format!("Transfer Test Account 2 {}", Uuid::new_v4()))
        .email(format!("transfer2_{}@example.com", Uuid::new_v4()))
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
        .expect("Failed to create account 2");

    println!("✅ Accounts created:");
    println!(
        "   - Account 1 ID: {} (Balance: ${})",
        account1.id, account1.balance
    );
    println!(
        "   - Account 2 ID: {} (Balance: ${})",
        account2.id, account2.balance
    );

    // === STEP 2: UPDATE ACCOUNTS WITH BALANCE ===
    println!("\n💰 STEP 2: Updating accounts with balance...");

    let updated_account1 = AccountBuilder::new()
        .id(account1.id)
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
        .expect("Failed to update account 1 balance");

    let updated_account2 = AccountBuilder::new()
        .id(account2.id)
        .balance(500.0)
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
        .expect("Failed to update account 2 balance");

    println!("✅ Accounts updated:");
    println!("   - Account 1 Balance: ${}", updated_account1.balance);
    println!("   - Account 2 Balance: ${}", updated_account2.balance);

    // === STEP 3: CREATE TRANSFER TRANSACTION ===
    println!("\n🔄 STEP 3: Creating Transfer transaction...");

    // Generate a unique parent_tx_key that will link all three transactions
    let parent_tx_key = format!("txgroup_{}", Uuid::new_v4());
    let transfer_amount = 200.0;

    let transfer_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Transfer)
        .from_account_id(account1.id)
        .to_account_id(account2.id)
        .amount(transfer_amount)
        .currency("USD".to_string())
        .idempotency_key(format!("idem_transfer_{}", Uuid::new_v4()))
        .parent_tx_key(parent_tx_key.clone())
        .description("Transfer from Account 1 to Account 2".to_string())
        .expect_id()
        .expect_transaction_type()
        .expect_from_account_id()
        .expect_to_account_id()
        .expect_amount()
        .expect_status()
        .expect_parent_tx_key()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create transfer transaction");

    println!("✅ Transfer transaction created:");
    println!("   - ID: {}", transfer_txn.id);
    println!("   - Amount: ${}", transfer_txn.amount);
    println!("   - Status: {:?}", transfer_txn.status);
    println!("   - Parent TX Key: {}", transfer_txn.parent_tx_key);

    assert_eq!(transfer_txn.transaction_type, TransactionType::Transfer);
    assert_eq!(transfer_txn.from_account_id, Some(account1.id));
    assert_eq!(transfer_txn.to_account_id, Some(account2.id));
    assert_eq!(transfer_txn.parent_tx_key, parent_tx_key);

    // === STEP 4: CREATE DEBIT TRANSACTION (with same parent_tx_key) ===
    println!("\n💸 STEP 4: Creating Debit transaction (linked to Transfer)...");

    let debit_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Debit)
        .from_account_id(account1.id)
        .amount(transfer_amount)
        .currency("USD".to_string())
        .idempotency_key(format!("idem_debit_{}", Uuid::new_v4()))
        .parent_tx_key(parent_tx_key.clone()) // Same parent_tx_key as Transfer
        .description("Debit for transfer".to_string())
        .expect_id()
        .expect_transaction_type()
        .expect_from_account_id()
        .expect_amount()
        .expect_status()
        .expect_parent_tx_key()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create debit transaction");

    println!("✅ Debit transaction created:");
    println!("   - ID: {}", debit_txn.id);
    println!("   - Amount: ${}", debit_txn.amount);
    println!("   - Status: {:?}", debit_txn.status);
    println!("   - Parent TX Key: {}", debit_txn.parent_tx_key);

    assert_eq!(debit_txn.transaction_type, TransactionType::Debit);
    assert_eq!(debit_txn.from_account_id, Some(account1.id));
    assert_eq!(debit_txn.parent_tx_key, parent_tx_key);

    // === STEP 5: CREATE CREDIT TRANSACTION (with same parent_tx_key) ===
    println!("\n💰 STEP 5: Creating Credit transaction (linked to Transfer)...");

    let credit_txn = TransactionBuilder::new()
        .transaction_type(TransactionType::Credit)
        .to_account_id(account2.id)
        .amount(transfer_amount)
        .currency("USD".to_string())
        .idempotency_key(format!("idem_credit_{}", Uuid::new_v4()))
        .parent_tx_key(parent_tx_key.clone()) // Same parent_tx_key as Transfer and Debit
        .description("Credit for transfer".to_string())
        .expect_id()
        .expect_transaction_type()
        .expect_to_account_id()
        .expect_amount()
        .expect_status()
        .expect_parent_tx_key()
        .create(Some(&mut conn))
        .await
        .expect("Failed to create credit transaction");

    println!("✅ Credit transaction created:");
    println!("   - ID: {}", credit_txn.id);
    println!("   - Amount: ${}", credit_txn.amount);
    println!("   - Status: {:?}", credit_txn.status);
    println!("   - Parent TX Key: {}", credit_txn.parent_tx_key);

    assert_eq!(credit_txn.transaction_type, TransactionType::Credit);
    assert_eq!(credit_txn.to_account_id, Some(account2.id));
    assert_eq!(credit_txn.parent_tx_key, parent_tx_key);

    // === VERIFICATION ===
    println!("\n✅ VERIFICATION: All three transactions share the same parent_tx_key:");
    println!(
        "   - Transfer parent_tx_key: {}",
        transfer_txn.parent_tx_key
    );
    println!("   - Debit parent_tx_key:    {}", debit_txn.parent_tx_key);
    println!("   - Credit parent_tx_key:   {}", credit_txn.parent_tx_key);

    assert_eq!(transfer_txn.parent_tx_key, debit_txn.parent_tx_key);
    assert_eq!(debit_txn.parent_tx_key, credit_txn.parent_tx_key);

   
    db_ops.tracker().return_connection(conn);
    db_ops.shutdown().await;

    println!("\n=== ✅ TEST COMPLETED SUCCESSFULLY ===");
    println!(
        "All transactions properly linked with parent_tx_key: {}",
        parent_tx_key
    );
}
