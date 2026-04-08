-- Core Schema
-- No cloud services, pure local SQLite

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    email TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,
    avatar TEXT DEFAULT '',
    signature TEXT DEFAULT '',
    group_id INTEGER NOT NULL DEFAULT 3,  -- 1=admin, 2=moderator, 3=member
    post_count INTEGER NOT NULL DEFAULT 0,
    thread_count INTEGER NOT NULL DEFAULT 0,
    credits INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 1,  -- 1=normal, 0=banned
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS forums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    description TEXT DEFAULT '',
    sort_order INTEGER NOT NULL DEFAULT 0,
    parent_id INTEGER DEFAULT NULL,
    thread_count INTEGER NOT NULL DEFAULT 0,
    post_count INTEGER NOT NULL DEFAULT 0,
    last_thread_id INTEGER DEFAULT NULL,
    last_post_at TEXT DEFAULT NULL,
    last_post_user TEXT DEFAULT '',
    status INTEGER NOT NULL DEFAULT 1,  -- 1=normal, 0=hidden
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS threads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    forum_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    is_top INTEGER NOT NULL DEFAULT 0,  -- 0=normal, 1=sticky
    is_closed INTEGER NOT NULL DEFAULT 0,
    view_count INTEGER NOT NULL DEFAULT 0,
    reply_count INTEGER NOT NULL DEFAULT 0,
    last_post_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_post_user TEXT DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (forum_id) REFERENCES forums(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS posts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id INTEGER NOT NULL,
    forum_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    content TEXT NOT NULL,
    floor INTEGER NOT NULL DEFAULT 0,  -- floor number in thread
    is_first INTEGER NOT NULL DEFAULT 0,  -- 1=original post
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (thread_id) REFERENCES threads(id),
    FOREIGN KEY (forum_id) REFERENCES forums(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    expires_at TEXT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Insert default settings
INSERT OR IGNORE INTO settings (key, value) VALUES ('site_name', 'RustForum');
INSERT OR IGNORE INTO settings (key, value) VALUES ('site_description', 'A modern forum system built with Rust + Axum + SQLite');
INSERT OR IGNORE INTO settings (key, value) VALUES ('site_keywords', 'forum,rust,axum,sqlite');
INSERT OR IGNORE INTO settings (key, value) VALUES ('site_footer_text', 'Powered by RustForum');
INSERT OR IGNORE INTO settings (key, value) VALUES ('posts_per_page', '20');
INSERT OR IGNORE INTO settings (key, value) VALUES ('threads_per_page', '30');
INSERT OR IGNORE INTO settings (key, value) VALUES ('setup_completed', '0');

-- Insert default forum
INSERT OR IGNORE INTO forums (id, name, description, sort_order) VALUES (1, '综合讨论', '综合话题讨论区', 1);
INSERT OR IGNORE INTO forums (id, name, description, sort_order) VALUES (2, '技术交流', '技术话题与经验分享', 2);
INSERT OR IGNORE INTO forums (id, name, description, sort_order) VALUES (3, '站务公告', '站点公告与反馈', 0);

-- Performance indexes
CREATE INDEX IF NOT EXISTS idx_threads_forum ON threads(forum_id, is_top DESC, last_post_at DESC);
CREATE INDEX IF NOT EXISTS idx_threads_user ON threads(user_id);
CREATE INDEX IF NOT EXISTS idx_posts_thread ON posts(thread_id, floor);
CREATE INDEX IF NOT EXISTS idx_posts_user ON posts(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
