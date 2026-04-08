// Setup guard middleware
// Redirects all requests to /setup if the installation wizard has not been completed

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use crate::config::AppState;

pub async fn check_setup(
    State(_state): State<AppState>,
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let path = req.uri().path();

    // Allow setup routes and static files through
    if path.starts_with("/setup") || path.starts_with("/static") {
        return next.run(req).await;
    }

    // Check if setup is completed
    if !crate::site_config::is_setup_completed() {
        return (StatusCode::FOUND, [("Location", "/setup")]).into_response();
    }

    next.run(req).await
}
