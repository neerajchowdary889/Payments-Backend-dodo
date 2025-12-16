use payments_backend_dodo::datalayer::{DbConfig, DbManager, initialize_database};
use std::time::Duration;
use tracing_subscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load environment variables
    dotenvy::dotenv().ok();

    println!("🚀 Starting Payments Backend Application...");

    // Initialize database with default configuration
    // This is idempotent - can be called multiple times safely
    let db_manager = initialize_database().await?;

    println!("✅ Database initialized successfully!");

    // Perform health check
    match db_manager.health_check().await {
        Ok(health) => {
            println!("💚 Database Health Check:");
            println!("   - Status: Healthy");
            println!("   - Latency: {}ms", health.latency_ms);
            println!("   - Pool Size: {}", health.pool_size);
            println!("   - Idle Connections: {}", health.idle_connections);
        }
        Err(e) => {
            eprintln!("❌ Database health check failed: {}", e);
            return Err(e.into());
        }
    }

    // Example: Custom configuration
    // let custom_config = DbConfig {
    //     database_url: "postgres://user:pass@localhost:5432/mydb".to_string(),
    //     max_connections: 20,
    //     min_connections: 5,
    //     connection_timeout: Duration::from_secs(30),
    //     idle_timeout: Duration::from_secs(600),
    //     max_lifetime: Duration::from_secs(1800),
    // };
    // let db_manager = DbManager::new(custom_config).await?;

    // Get pool reference for use in application
    let pool = db_manager.pool();

    // Example query
    let result: (i64,) = sqlx::query_as("SELECT $1")
        .bind(42_i64)
        .fetch_one(pool)
        .await?;

    println!("✅ Test query result: {}", result.0);

    println!("🎯 Application ready to handle requests!");

    // In a real application, you would:
    // 1. Start your web server (Axum)
    // 2. Pass db_manager to your application state
    // 3. Use it in your handlers

    // Example: Graceful shutdown
    // tokio::signal::ctrl_c().await?;
    // db_manager.shutdown().await;

    Ok(())
}
