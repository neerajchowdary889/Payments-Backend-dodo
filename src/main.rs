use payments_backend_dodo::{
    datalayer::initialize_database, logging::init_telemetry, routes::create_router,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables first
    dotenvy::dotenv().ok();

    // Initialize OpenTelemetry (tracing, metrics, and logging)
    init_telemetry(None)?;

    tracing::info!("Starting Payments Backend Application");

    // Initialize database with default configuration
    // This is idempotent - can be called multiple times safely
    let db_ops = initialize_database().await?;

    tracing::info!("Database initialized successfully");

    // Create router with all routes
    let app = create_router();

    // Get server address from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Log server startup with structured telemetry
    tracing::info!(
        address = %addr,
        port = %port,
        "Server listening and ready to accept connections"
    );

    tracing::info!(
        endpoints = ?vec![
            "/health - health check",
        ],
        "Available API endpoints"
    );

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

    tracing::warn!("Shutdown signal received, cleaning up...");
}
