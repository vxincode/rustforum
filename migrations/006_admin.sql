-- Admin system tables

-- Reports table
CREATE TABLE IF NOT EXISTS reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    reporter_id INTEGER NOT NULL,
    target_type TEXT NOT NULL,       -- 'thread' / 'post' / 'user'
    target_id INTEGER NOT NULL,
    reason TEXT NOT NULL,            -- report reason
    description TEXT DEFAULT '',     -- detailed description
    status TEXT NOT NULL DEFAULT 'pending', -- pending / reviewing / resolved / dismissed
    admin_id INTEGER,                -- handling admin
    admin_note TEXT DEFAULT '',      -- admin note
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_reports_status ON reports(status, created_at DESC);

-- Blacklist (IP + user ban records)
CREATE TABLE IF NOT EXISTS blacklist (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    type TEXT NOT NULL,              -- 'ip' / 'user'
    value TEXT NOT NULL,             -- IP address or user ID
    reason TEXT DEFAULT '',
    admin_id INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(type, value)
);

-- Muted users
CREATE TABLE IF NOT EXISTS muted_users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL UNIQUE,
    reason TEXT DEFAULT '',
    admin_id INTEGER,
    expires_at TEXT,                 -- NULL = permanent
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Invite codes
CREATE TABLE IF NOT EXISTS invite_codes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    code TEXT NOT NULL UNIQUE,
    created_by INTEGER NOT NULL,
    max_uses INTEGER NOT NULL DEFAULT 1,
    used_count INTEGER NOT NULL DEFAULT 0,
    expires_at TEXT,                 -- NULL = never expires
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_invite_codes_code ON invite_codes(code);
