use axum::{
    Router,
    routing::{get, patch, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::handlers::{accounts, health, transfer};

/// Create the main application router with all routes and middleware
pub fn create_router() -> Router {
    // Health routes
    let health_routes = Router::new().route("/health", get(health::health_check));

    // Account routes - /api/v1/accounts
    let account_routes = Router::new()
        .route("/api/v1/accounts", post(accounts::create_account))
        .route("/api/v1/accounts/:id", get(accounts::get_account))
        .route("/api/v1/accounts/:id", patch(accounts::update_account))
        .route("/api/v1/accounts/:id/balance", get(accounts::get_balance));

    // Transfer routes - /api/v1/transfer
    let transfer_routes = Router::new()
        .route("/api/v1/transfer", post(transfer::transfer))
        .route("/api/v1/transfer/:id", get(transfer::get_transfer))
        .route("/api/v1/transfer", get(transfer::list_transfers));

    // Main router combining all routes
    Router::new()
        .merge(health_routes)
        .merge(account_routes)
        .merge(transfer_routes)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}
