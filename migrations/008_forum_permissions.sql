-- 版块权限系统
-- view_perm: 浏览权限 (0=所有人, 1=登录用户, 2=版主+, 3=仅管理员)
-- post_perm: 发帖权限 (0=所有用户, 1=版主+, 2=仅管理员)
-- forum_moderators: 版块版主分配表

CREATE TABLE IF NOT EXISTS forum_moderators (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    forum_id INTEGER NOT NULL,
    user_id INTEGER NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(forum_id, user_id)
);
