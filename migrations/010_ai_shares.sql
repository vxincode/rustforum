-- AI 共享模块：AI Prompt / Skill 分享表
CREATE TABLE IF NOT EXISTS ai_shares (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    content TEXT NOT NULL,
    category TEXT NOT NULL,
    share_type TEXT NOT NULL DEFAULT 'prompt',
    price INTEGER NOT NULL DEFAULT 0,
    download_count INTEGER NOT NULL DEFAULT 0,
    status INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
CREATE INDEX IF NOT EXISTS idx_ai_shares_category ON ai_shares(category);
CREATE INDEX IF NOT EXISTS idx_ai_shares_user ON ai_shares(user_id);

-- AI 共享购买记录表
CREATE TABLE IF NOT EXISTS ai_share_purchases (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    share_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    credits_paid INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(share_id, user_id),
    FOREIGN KEY (share_id) REFERENCES ai_shares(id),
    FOREIGN KEY (user_id) REFERENCES users(id)
);
