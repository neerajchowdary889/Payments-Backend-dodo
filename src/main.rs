use payments_backend_dodo::{
    datalayer::initialize_database, logging::init_telemetry, routes::create_router, state::AppState,
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
    let _db_ops = initialize_database().await?;

    tracing::info!("Database initialized successfully");

    // Initialize AppState with Redis
    let redis_url = std::env::var("REDIS_URL").unwrap_or_else(|_| {
        tracing::warn!("REDIS_URL not set, using default: redis://localhost:6379");
        "redis://localhost:6379".to_string()
    });

    tracing::info!(redis_url = %redis_url, "Connecting to Redis");

    let app_state = AppState::new(&redis_url)
        .await
        .expect("Failed to initialize AppState with Redis");

    tracing::info!("Redis connection established successfully");

    // Create router with all routes and app state
    let app = create_router(app_state);

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
            "/api/v1/accounts - account management",
            "/api/v1/transfer - transfer operations",
        ],
        "Available API endpoints"
    );

    // Start the server with graceful shutdown
    // Use into_make_service_with_connect_info to provide socket address for IP rate limiting
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
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
