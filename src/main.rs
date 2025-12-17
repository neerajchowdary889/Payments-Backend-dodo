use payments_backend_dodo::{
    datalayer::initialize_database, logging::init_telemetry, routes::create_router,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables first
    dotenvy::dotenv().ok();

    // Initialize OpenTelemetry (tracing, metrics, and logging)
    init_telemetry(None)?;

    println!("🚀 Starting Payments Backend Application...");

    // Initialize database with default configuration
    // This is idempotent - can be called multiple times safely
    let db_ops = initialize_database().await?;

    println!("✅ Database initialized successfully!");

    // Create router with all routes
    let app = create_router();

    // Get server address from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    println!("🎯 Server listening on http://{}", addr);
    println!("📋 Available endpoints:");
    println!("   - GET  /health           - Comprehensive health check");
    println!("   - GET  /health/liveness  - Liveness probe");
    println!("   - GET  /health/readiness - Readiness probe");

    // Start the server with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Shutdown telemetry gracefully
    payments_backend_dodo::logging::shutdown_telemetry();

    Ok(())
}

/// Handle graceful shutdown signals
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n🛑 Shutdown signal received, cleaning up...");
}
