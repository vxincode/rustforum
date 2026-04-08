// Installation setup wizard handler
// Provides a 3-step guided setup: admin account → site settings → complete

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
    Form,
};
use serde::Deserialize;
use sqlx::SqlitePool;

use crate::config::AppState;
use crate::templates;

#[derive(Deserialize)]
pub struct SetupForm {
    step: i32,
    username: Option<String>,
    email: Option<String>,
    password: Option<String>,
    password_confirm: Option<String>,
    site_name: Option<String>,
    site_description: Option<String>,
    site_keywords: Option<String>,
}

// GET /setup — render setup wizard page
pub async fn setup_page(
    State(_state): State<AppState>,
) -> impl IntoResponse {
    // If already completed, redirect to home
    if crate::site_config::is_setup_completed() {
        return Redirect::to("/").into_response();
    }
    Html(templates::render_setup(1, "")).into_response()
}

// POST /setup — handle setup form submission
pub async fn setup_submit(
    State(state): State<AppState>,
    Form(form): Form<SetupForm>,
) -> impl IntoResponse {
    // If already completed, redirect to home
    if crate::site_config::is_setup_completed() {
        return Redirect::to("/").into_response();
    }

    match form.step {
        1 => handle_step1(&state.pool, &form).await,
        2 => handle_step2(&state.pool, &form).await,
        _ => Html(templates::render_setup(1, "Invalid step")).into_response(),
    }
}

async fn handle_step1(pool: &SqlitePool, form: &SetupForm) -> axum::response::Response {
    let username = form.username.as_deref().unwrap_or("").trim();
    let email = form.email.as_deref().unwrap_or("").trim();
    let password = form.password.as_deref().unwrap_or("");
    let password_confirm = form.password_confirm.as_deref().unwrap_or("");

    // Validate
    if username.is_empty() || email.is_empty() || password.is_empty() {
        return Html(templates::render_setup(1, "All fields are required")).into_response();
    }
    if username.len() < 2 || username.len() > 20 {
        return Html(templates::render_setup(1, "Username must be 2-20 characters")).into_response();
    }
    if !email.contains('@') {
        return Html(templates::render_setup(1, "Invalid email address")).into_response();
    }
    if password.len() < 6 {
        return Html(templates::render_setup(1, "Password must be at least 6 characters")).into_response();
    }
    if password != password_confirm {
        return Html(templates::render_setup(1, "Passwords do not match")).into_response();
    }

    // Check username uniqueness
    let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .unwrap_or(None);
    if exists.is_some() {
        return Html(templates::render_setup(1, "Username already taken")).into_response();
    }

    // Create admin account
    let hash = match bcrypt::hash(password, bcrypt::DEFAULT_COST) {
        Ok(h) => h,
        Err(_) => return Html(templates::render_setup(1, "Failed to hash password")).into_response(),
    };

    let result = sqlx::query(
        "INSERT INTO users (username, email, password_hash, group_id, status) VALUES (?, ?, ?, 1, 1)"
    )
    .bind(username)
    .bind(email)
    .bind(&hash)
    .execute(pool)
    .await;

    match result {
        Ok(_) => Html(templates::render_setup(2, "")).into_response(),
        Err(e) => Html(templates::render_setup(1, &format!("Failed to create admin: {}", e))).into_response(),
    }
}

async fn handle_step2(pool: &SqlitePool, form: &SetupForm) -> axum::response::Response {
    let site_name = form.site_name.as_deref().unwrap_or("").trim();
    let site_description = form.site_description.as_deref().unwrap_or("").trim();
    let site_keywords = form.site_keywords.as_deref().unwrap_or("").trim();

    if site_name.is_empty() {
        return Html(templates::render_setup(2, "Site name is required")).into_response();
    }

    // Update site settings
    let _ = sqlx::query("UPDATE settings SET value = ? WHERE key = 'site_name'")
        .bind(site_name)
        .execute(pool)
        .await;
    let _ = sqlx::query("UPDATE settings SET value = ? WHERE key = 'site_description'")
        .bind(if site_description.is_empty() { "A modern forum system" } else { site_description })
        .execute(pool)
        .await;
    let _ = sqlx::query("UPDATE settings SET value = ? WHERE key = 'site_keywords'")
        .bind(if site_keywords.is_empty() { "forum,rust,axum,sqlite" } else { site_keywords })
        .execute(pool)
        .await;

    // Mark setup as completed
    let _ = sqlx::query("UPDATE settings SET value = '1' WHERE key = 'setup_completed'")
        .execute(pool)
        .await;

    // Refresh global cache
    crate::site_config::load_from_db(pool).await;
    crate::site_config::set_setup_completed(true);

    Html(templates::render_setup(3, "")).into_response()
}
