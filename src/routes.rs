use axum::{
    Router, middleware,
    routing::{get, patch, post},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    handlers::{accounts, health, transfer, webhooks},
    middleware::{
        auth::auth_middleware, ip_rate_limit::ip_rate_limit_middleware,
        rate_limit::rate_limit_middleware, request_id::request_id_middleware,
    },
    state::AppState,
};

/// Create the main application router with all routes and middleware
pub fn create_router(state: AppState) -> Router {
    // Public routes (no authentication required, IP-based rate limiting)
    let public_routes = Router::new()
        .route("/health", get(health::health_check))
        .route("/api/v1/accounts", post(accounts::create_account))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            ip_rate_limit_middleware,
        ));

    // Protected routes (authentication required, API-key-based rate limiting)
    let protected_routes_accounts = Router::new()
        .route("/api/v1/accounts/putbalance", post(accounts::put_balance))
        .route("/api/v1/accounts/:id", get(accounts::get_account))
        .route("/api/v1/accounts/:id", patch(accounts::update_account))
        .route("/api/v1/accounts/:id/balance", get(accounts::get_balance));

    let protected_routes_webhooks = Router::new()
        .route("/api/v1/webhooks/set", post(webhooks::create_webhook))
        .route("/api/v1/webhooks/unset", post(webhooks::delete_webhook))
        .route("/api/v1/webhooks/info", get(webhooks::get_webhooks));

    let protected_routes_transfer = Router::new()
        .route("/api/v1/transfer", post(transfer::transfer))
        .route("/api/v1/transfer/list", get(transfer::list_transfers))
        .route(
            "/api/v1/transfer/info/:id",
            get(transfer::get_transfer_byid),
        )
        .route(
            "/api/v1/transfer/:id",
            get(transfer::get_transfer_byparentkey),
        );

    let protected_routes = Router::new()
        .merge(protected_routes_accounts)
        .merge(protected_routes_webhooks)
        .merge(protected_routes_transfer)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Combine all routes with shared middleware
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .layer(middleware::from_fn(request_id_middleware))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
