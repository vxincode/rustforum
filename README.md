# RustForum

A modern, high-performance forum system built with **Rust + Axum + SQLite**. Server-side rendered, single binary deployment, zero external service dependencies.

## Features

- **User System** — Registration, login, profiles, avatar upload, daily check-in, credits
- **Forum Core** — Multi-forum categories, threads, replies, Markdown rendering, sticky/essence/close
- **Interactions** — Private messaging, notifications (reply/quote/message), reporting
- **Admin Panel** — Dashboard, forum/user/thread management, login logs, invite codes, blacklist, AI review, backups
- **AI Share** — Share and purchase prompts/skills with credits
- **Mobile Responsive** — Adapts to all screen sizes

## Quick Start

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. Clone and build
git clone https://github.com/yourname/rustforum.git
cd rustforum
cargo build --release

# 3. Run
./target/release/rustforum
```

Open `http://localhost:3000` and follow the setup wizard to create your admin account and configure the site.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Web Framework | Axum 0.8 |
| Database | SQLite (via SQLx 0.8) |
| Markdown | pulldown-cmark 0.13 |
| Auth | bcrypt + cookie sessions |
| Frontend | Tailwind CSS + vanilla JS |
| Caching | Redis (optional, graceful degradation) |

## Configuration

All settings are read from environment variables (or a `.env` file):

| Variable | Default | Description |
|----------|---------|-------------|
| `LISTEN_ADDR` | `0.0.0.0:3000` | Listen address |
| `DATABASE_URL` | `sqlite:forum.db?mode=rwc` | SQLite database path |
| `SESSION_SECRET` | `rustforum-secret-change-me` | Session secret (**change in production**) |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis URL (optional) |

## Project Structure

```
src/
  main.rs           # Entry point: DB init, routes, HTTP server
  config.rs         # App configuration & state
  db.rs             # Database pool, migrations, seed data
  site_config.rs    # Global site settings cache
  templates.rs      # All HTML rendering (~4000 lines)
  cache.rs          # Redis cache helpers
  handlers/         # Request handlers by feature
  middleware/        # Auth, CSRF, rate limiting, setup guard
  models/           # Data models
migrations/         # SQL migration files
static/             # CSS, JS, fonts, images, avatars
```

## Deployment

See [DEPLOY.md](DEPLOY.md) for production deployment with Nginx reverse proxy.

## License

MIT
