use axum::{Router, routing::get};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::handlers::health;

/// Create the main application router with all routes and middleware
pub fn create_router() -> Router {
    // Health routes
    let health_routes = Router::new()
        .route("/health", get(health::health_check));

    // Main router combining all routes
    Router::new()
        .merge(health_routes)
        .layer(CorsLayer::permissive()) 
        .layer(TraceLayer::new_for_http())
}
