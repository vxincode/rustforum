# RustForum Development Skill

You are an expert Rust web developer continuing development on the RustForum project — a forum system built with Axum 0.8 + SQLite + SSR.

Use these rules to ensure all code follows the project's established patterns and conventions.

---

## Architecture Overview

```
Request → Axum Router → Middleware (auth/csrf/rate-limit/setup-guard)
       → Handler (extracts State, User, params)
       → SQLx query on SQLite
       → templates.rs render_*() → HTML response
```

- **No frontend framework** — everything is server-side rendered HTML
- **No template engine** — HTML is built with Rust `format!()` macros
- **Single binary** — static files served from `static/` directory
- **SQLite only** — no external database required
- **Redis optional** — graceful degradation when unavailable

---

## File Organization

| When adding... | Do this |
|---|---|
| New feature page | Add handler in `src/handlers/`, template fn in `src/templates.rs`, route in `src/main.rs` |
| New API endpoint | Add handler in `src/handlers/api.rs`, route in `src/main.rs` under `// JSON API` |
| New database table | Create `migrations/0xx_name.sql` with `CREATE TABLE IF NOT EXISTS`, register in `src/db.rs` `ddl_migrations` array |
| New column on existing table | Add entry to `src/db.rs` `alter_migrations` array as `(table, column, type, default)` |
| New middleware | Create `src/middleware/name.rs`, add `pub mod name` to `src/middleware/mod.rs` |
| New data model | Create `src/models/name.rs`, add `pub mod name` to `src/models/mod.rs`, derive `Debug, Clone, Serialize, Deserialize, FromRow` |

---

## Handler Pattern

All handlers follow this signature pattern:

```rust
// Page handler (SSR HTML)
pub async fn page_handler(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,          // optional auth
    // or AuthUser(user): AuthUser,       // required auth
    // or AdminUser(user): AdminUser,     // admin only
) -> impl IntoResponse {
    let pool = &state.pool;

    // 1. Fetch data
    let data: Vec<SomeModel> = sqlx::query_as("SELECT * FROM table WHERE ...")
        .bind(param)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    // 2. Render template
    let content = templates::render_some_page(&data);
    Html(content).into_response()
}

// Form POST handler
pub async fn form_handler(
    State(state): State<AppState>,
    AuthUser(user): AuthUser,
    Form(form): Form<SomeForm>,          // #[derive(Deserialize)]
) -> impl IntoResponse {
    let pool = &state.pool;

    // 1. Validate input
    if form.field.is_empty() {
        return Html(templates::render_error("Field is required")).into_response();
    }

    // 2. Execute database operation
    let result = sqlx::query("INSERT INTO table (col) VALUES (?)")
        .bind(&form.field)
        .execute(pool)
        .await;

    match result {
        Ok(_) => Redirect::to("/success-page").into_response(),
        Err(e) => Html(templates::render_error(&format!("Failed: {}", e))).into_response(),
    }
}

// JSON API handler
pub async fn api_handler(
    State(state): State<AppState>,
    MaybeUser(user): MaybeUser,
) -> impl IntoResponse {
    let data = fetch_data(&state.pool).await;
    Json(serde_json::json!({ "ok": true, "data": data })).into_response()
}
```

### Key conventions:
- Always use `State(state): State<AppState>` as first extractor
- Access database via `&state.pool`
- Return `impl IntoResponse` from all handlers
- Use `Html(...)` for page responses, `Json(...)` for API responses
- Use `Redirect::to(...)` for post-form redirects

---

## Template Pattern

All HTML is generated in `src/templates.rs` using `format!()` with raw string literals:

```rust
pub fn render_some_page(data: &[SomeModel], user: Option<&User>) -> String {
    let site_name = crate::site_config::site_name();

    let content = format!(r#"
<div class="container mx-auto px-4 py-8 max-w-5xl">
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 fade-in">
    <h1 class="text-2xl font-bold mb-4">{title}</h1>
    <div class="space-y-3">
        {items}
    </div>
  </div>
</div>"#,
        title = html_escape(&some_title),
        items = data.iter().map(|item| format!(
            r#"<div class="p-3 hover:bg-gray-50 rounded-lg">
                <span class="text-gray-800">{name}</span>
            </div>"#,
            name = html_escape(&item.name),
        )).collect::<Vec<_>>().join("\n"),
    );

    layout("Page Title", user, &content, "nav_section")
}
```

### Template rules:
- **Always** use `html_escape()` for any user-provided or dynamic text
- Use `layout(title, user, content, nav_section)` for standard pages
- Use `admin_layout(title, user, content)` for admin pages
- Use raw `format!()` with `r#"..."#` or `r##"..."##` for HTML containing `{` / `}`
- Tailwind CSS utility classes for all styling — no custom CSS classes unless necessary
- FontAwesome icons via `<i class="fa fa-icon-name"></i>`
- Standard card container: `bg-white rounded-xl border border-gray-100 shadow-sm p-6`

---

## Database Query Pattern

All queries use SQLx with parameterized binding:

```rust
// SELECT single row
let user: Option<User> = sqlx::query_as(
    "SELECT * FROM users WHERE id = ? AND status = 1"
)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

// SELECT multiple rows
let threads: Vec<Thread> = sqlx::query_as(
    "SELECT * FROM threads WHERE forum_id = ? ORDER BY is_top DESC, last_post_at DESC LIMIT ? OFFSET ?"
)
    .bind(forum_id)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

// INSERT
let result = sqlx::query(
    "INSERT INTO table (col1, col2) VALUES (?, ?)"
)
    .bind(&val1)
    .bind(&val2)
    .execute(pool)
    .await?;

// UPDATE (non-critical, ignore errors)
sqlx::query("UPDATE forums SET thread_count = thread_count + 1 WHERE id = ?")
    .bind(forum_id)
    .execute(pool)
    .await
    .ok();
```

### Query rules:
- **Always** use `?` placeholders with `.bind()` — never interpolate values into SQL
- Use `.await?` for critical operations, `.ok()` for non-critical
- Use `datetime('now')` for timestamps in SQLite
- Use `.unwrap_or_default()` for fetch_all, `.ok().flatten()` for fetch_optional

---

## Model Pattern

```rust
// Database model
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SomeModel {
    pub id: i64,
    pub name: String,
    pub created_at: String,
}

// Form input model
#[derive(Debug, Deserialize)]
pub struct SomeForm {
    pub name: String,
    pub description: Option<String>,
}
```

---

## Middleware Pattern

Custom extractors implement `FromRequestParts<AppState>`:

```rust
pub struct SomeExtractor(pub SomeType);

impl FromRequestParts<AppState> for SomeExtractor {
    type Rejection = axum::response::Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        // Check condition
        // Return Ok(SomeExtractor(value)) on success
        // Return Err(Redirect::to("/some-path").into_response()) on failure
        todo!()
    }
}
```

Available extractors:
- `MaybeUser(Option<User>)` — always succeeds, user may be None
- `AuthUser(User)` — redirects to `/auth/login` if not logged in
- `AdminUser(User)` — returns 403 if not admin

---

## Route Registration

Routes are registered in `src/main.rs` in this order:

```rust
let app = Router::new()
    .route("/setup", get(handlers::setup::setup_page).post(handlers::setup::setup_submit))
    .route("/", get(handlers::index::index))
    .route("/path/{param_id}", get(handler).post(handler))
    .route("/api/something", get(api_handler))
    .nest_service("/static", ServeDir::new("static"))
    .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::setup_guard::check_setup))
    .layer(TraceLayer::new_for_http())
    .with_state(state);
```

---

## Styling Reference

### Tailwind classes used throughout:
- Card: `bg-white rounded-xl border border-gray-100 shadow-sm p-6`
- Page header: `text-2xl font-bold text-gray-900 mb-2`
- Subtitle: `text-sm text-gray-500`
- Button primary: `bg-blue-600 text-white px-4 py-2 rounded-lg hover:bg-blue-700 transition`
- Button danger: `bg-red-600 text-white px-4 py-2 rounded-lg hover:bg-red-700 transition`
- Input field: `w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none`
- Error alert: `bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg`
- Success alert: `bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg`
- Table row: `hover:bg-gray-50 transition`
- Badge: `px-2 py-0.5 rounded-full text-xs font-medium`
- Container: `container mx-auto px-4 py-8 max-w-5xl`
- Fade-in animation: `fade-in` class

### Color scheme:
- Primary: blue-600 (#2563eb)
- Background: gray-50 (#f9fafb)
- Text: gray-900 (#111827) / gray-500 (#6b7280)
- Border: gray-100 (#f3f4f6) / gray-200 (#e5e7eb)
- Admin nav: dark sidebar with gray-800 background

---

## Common Patterns

### Pagination
```rust
let offset = (page - 1) * per_page;
let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM table WHERE ...")
    .bind(param).fetch_one(pool).await.unwrap_or((0,));
let total_pages = ((total.0 as f64) / per_page as f64).ceil() as i64;
```

### Session / Cookie
```rust
use axum::http::header::SET_COOKIE;
// Set cookie
(headers_mut).insert(SET_COOKIE, format!("session_id={}; HttpOnly; SameSite=Lax; Path=/; Max-Age=604800", session_id).parse().unwrap());
// Clear cookie
(headers_mut).insert(SET_COOKIE, "session_id=; HttpOnly; SameSite=Lax; Path=/; Max-Age=0".parse().unwrap());
```

### Password hashing
```rust
let hash = bcrypt::hash(password, bcrypt::DEFAULT_COST)?;
let valid = bcrypt::verify(password, &hash)?;
```

### CSRF token (in templates)
```html
<meta name="csrf-token" content="{csrf_token}">
```
The frontend JS in `static/js/app.js` automatically injects CSRF tokens into forms.

### Global site settings
```rust
let name = crate::site_config::site_name();       // "RustForum"
let desc = crate::site_config::site_description(); // "A modern..."
let footer = crate::site_config::site_footer();
```

### Redis caching (optional)
```rust
use crate::cache;
// Try cache first, fallback to DB
let data = cache::get_cached::<Vec<Forum>>(&state.redis, "cache:key").await;
if data.is_none() {
    let fresh = fetch_from_db(pool).await;
    cache::set_cached(&state.redis, "cache:key", &fresh, 300).await;
}
```

---

## Database Schema Conventions

- Tables use `INTEGER PRIMARY KEY AUTOINCREMENT` for IDs
- Timestamps stored as `TEXT DEFAULT (datetime('now'))`
- Status fields use integers: `1=normal, 0=disabled/banned`
- Foreign keys always defined with `REFERENCES table(id)`
- User groups: `1=admin, 2=moderator, 3=member`
- All migrations must be idempotent (`IF NOT EXISTS`, `INSERT OR IGNORE`)

---

## When adding a new complete feature, follow this checklist:

1. [ ] Create migration SQL file in `migrations/`
2. [ ] Register migration in `src/db.rs` `ddl_migrations` or `alter_migrations`
3. [ ] Create model in `src/models/` with `FromRow` derive
4. [ ] Add `pub mod` in `src/models/mod.rs`
5. [ ] Create handler file in `src/handlers/`
6. [ ] Add `pub mod` in `src/handlers/mod.rs`
7. [ ] Add `render_*` function(s) in `src/templates.rs`
8. [ ] Register routes in `src/main.rs`
9. [ ] Test with `cargo build` — no errors, no warnings
10. [ ] Delete `forum.db`, run fresh, verify setup wizard still works
