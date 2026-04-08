# Deployment Guide (BaoTa Panel)

## Requirements

- BaoTa Panel 7.x/8.x (Linux)
- CentOS 7+ / Ubuntu 18+ / Debian 10+
- RAM >= 1GB (for compilation)
- Rust toolchain

---

## 1. Install Rust

SSH into the server and run:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Select default install (option 1), then load the environment:

```bash
source $HOME/.cargo/env
```

Verify:

```bash
rustc --version
cargo --version
```

---

## 2. Upload Project

Upload the entire project to the server, e.g. `/www/rustforum/`.

Via BaoTa file manager or Git:

```bash
cd /www
git clone <your-repo-url> rustforum
```

Ensure the directory structure:

```
/www/rustforum/
├── Cargo.toml
├── src/
├── static/
│   ├── avatars/
│   ├── css/
│   ├── fonts/
│   ├── images/
│   └── js/
├── migrations/
└── .env          ← create later
```

---

## 3. Build Project

```bash
cd /www/rustforum

# Build release version (optimized)
cargo build --release
```

> First build downloads dependencies, takes 5-15 minutes depending on server specs.

The binary will be at:

```
target/release/rustforum
```

---

## 4. Create Environment Config

```bash
cd /www/rustforum
cat > .env << 'EOF'
# Listen address (local only, proxied by Nginx)
LISTEN_ADDR=127.0.0.1:3000

# Database path (absolute)
DATABASE_URL=sqlite:/www/rustforum/data/forum.db?mode=rwc

# Avatar directory (absolute)
AVATAR_DIR=/www/rustforum/static/avatars

# Avatar size limit (bytes)
MAX_AVATAR_SIZE=524288

# Session secret (CHANGE THIS to a random string)
SESSION_SECRET=change-me-to-a-random-string

# Site info
SITE_NAME=MyForum
SITE_DESC=A great forum
EOF
```

> **Important**: `SESSION_SECRET` must be changed to a random string for security.

Create data directory:

```bash
mkdir -p /www/rustforum/data
mkdir -p /www/rustforum/backups
```

---

## 5. Set Directory Permissions

```bash
chown -R www:www /www/rustforum
chmod -R 755 /www/rustforum
chmod -R 777 /www/rustforum/static/avatars
chmod -R 777 /www/rustforum/data
chmod -R 777 /www/rustforum/backups
```

---

## 6. Create systemd Service

```bash
cat > /etc/systemd/system/rustforum.service << 'EOF'
[Unit]
Description=RustForum
After=network.target

[Service]
Type=simple
User=www
Group=www
WorkingDirectory=/www/rustforum
EnvironmentFile=/www/rustforum/.env
ExecStart=/www/rustforum/target/release/rustforum
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
```

Start the service:

```bash
systemctl daemon-reload
systemctl enable rustforum
systemctl start rustforum
```

Check status:

```bash
systemctl status rustforum
```

See `active (running)` means success.

View logs:

```bash
journalctl -u rustforum -f
```

---

## 7. Configure Nginx Reverse Proxy (BaoTa)

### 7.1 Add Site in BaoTa

1. BaoTa Panel -> "Website" -> "Add Site"
2. Enter your domain (e.g. `forum.example.com`)
3. PHP version: "Pure Static"
4. No database needed
5. Click "Submit"

### 7.2 Configure Reverse Proxy

1. Click site name -> "Reverse Proxy" -> "Add Reverse Proxy"
2. Configure:
   - Proxy name: `rustforum`
   - Target URL: `http://127.0.0.1:3000`
   - Send domain: `$host`
3. Click "Save"

### 7.3 Edit Nginx Config (Important)

Click site -> "Config File", replace inside `server { }`:

```nginx
    # Reverse proxy to Rust backend
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Upload size limit (for backups)
        client_max_body_size 100m;

        # Timeout settings
        proxy_connect_timeout 60s;
        proxy_read_timeout 120s;
        proxy_send_timeout 120s;
    }

    # Static files served by Nginx (optional optimization)
    location /static/ {
        alias /www/rustforum/static/;
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
```

### 7.4 Configure SSL (HTTPS)

1. Site -> "SSL" -> "Let's Encrypt"
2. Select domain -> Apply
3. Enable "Force HTTPS"

---

## 8. Verify Deployment

1. Visit `https://your-domain/` in browser
2. You will be redirected to the setup wizard
3. Create admin account and configure site settings
4. Click "Enter Forum" to start using your forum

---

## 9. Maintenance

### Update and Redeploy

```bash
cd /www/rustforum
git pull                          # Pull latest code
cargo build --release             # Build
systemctl restart rustforum       # Restart service
```

### View Logs

```bash
# Live logs
journalctl -u rustforum -f

# Last 100 lines
journalctl -u rustforum -n 100
```

### Backup & Restore

In Admin Panel -> "Data Backup":
- Create backup: generates ZIP file
- Download backup: save to local
- Restore data: upload ZIP to restore

Manual database backup:

```bash
cp /www/rustforum/data/forum.db /www/backup/forum_$(date +%Y%m%d).db
```

### Database Maintenance

```bash
sqlite3 /www/rustforum/data/forum.db

# Clean expired sessions
DELETE FROM sessions WHERE expires_at < datetime('now');

# Exit
.quit
```

---

## 10. Troubleshooting

### Q: Build error `linker 'cc' not found`

Install build tools:

```bash
# CentOS
yum groupinstall -y "Development Tools"

# Ubuntu/Debian
apt install -y build-essential
```

### Q: Out of memory during build

Use swap:

```bash
fallocate -l 2G /swapfile
chmod 600 /swapfile
mkswap /swapfile
swapon /swapfile
```

Or cross-compile locally and upload the binary.

### Q: Cross-compile for Linux on Windows/Mac

```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

Then upload `target/x86_64-unknown-linux-gnu/release/rustforum` to the server.

### Q: 502 Bad Gateway after startup

Check if service is running:

```bash
systemctl status rustforum
journalctl -u rustforum -n 20
```

Common causes:
- `.env` file path incorrect
- Database directory doesn't exist
- Port in use (`ss -tlnp | grep 3000`)

### Q: Avatar upload fails

Check directory permissions:

```bash
chmod -R 777 /www/rustforum/static/avatars
chown -R www:www /www/rustforum/static/avatars
```

---

## Directory Structure Overview

```
/www/rustforum/
├── .env                          ← Environment config
├── target/release/rustforum      ← Binary
├── data/
│   └── forum.db                  ← SQLite database
├── static/
│   ├── avatars/                  ← User avatars (writable)
│   ├── css/
│   ├── fonts/
│   ├── images/
│   └── js/
├── backups/                      ← System backups (writable)
├── migrations/                   ← DB migration scripts
└── src/                          ← Source code
```
