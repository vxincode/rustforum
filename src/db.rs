// 数据库管理模块
// 职责：数据库连接池初始化、DDL/ALTER 迁移执行、种子数据填充、头像记录修复
// 迁移策略：DDL 使用 IF NOT EXISTS 保证幂等性，ALTER TABLE 先检查列是否存在再添加

use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

// 初始化 SQLite 连接池
// 启用外键约束（foreign_keys），确保数据库层面的引用完整性
pub async fn init_pool(database_url: &str) -> Result<SqlitePool, sqlx::Error> {
    let mut opts = SqliteConnectOptions::from_str(database_url)?;
    opts = opts
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5))
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal);
    let pool = SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(opts)
        .await?;
    Ok(pool)
}

// 检查表中是否存在指定列
// 通过 SQLite 的 pragma_table_info 查询表结构信息
/// Check if a column exists in a table
async fn column_exists(pool: &SqlitePool, table: &str, column: &str) -> bool {
    let result: Vec<(String,)> = sqlx::query_as(
        &format!("SELECT name FROM pragma_table_info('{}') WHERE name = '{}'", table, column)
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    !result.is_empty()
}

// 执行数据库迁移，分为两个阶段：
// 阶段一：执行 DDL（建表、建索引），所有语句使用 IF NOT EXISTS 保证幂等
// 阶段二：安全地 ALTER TABLE 添加新列，先检查列是否存在再执行
pub async fn run_migrations(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // 阶段一：执行建表和建索引的 SQL 迁移文件（所有语句均使用 IF NOT EXISTS，可重复执行）
    let ddl_migrations = [
        "migrations/001_init.sql",          // 初始表结构（用户、版块、帖子、回复等）
        "migrations/002_messages.sql",      // 私信表
        "migrations/003_notifications.sql", // 通知表
        "migrations/004_checkins.sql",      // 签到表
        "migrations/006_admin.sql",         // 管理相关表（举报、黑名单、邀请码、设置等）
        "migrations/007_login_logs.sql",    // 登录日志表
        "migrations/008_forum_permissions.sql", // 版块权限表
        "migrations/009_email_verification.sql", // 邮箱验证表
        "migrations/010_ai_shares.sql",           // AI 共享表
    ];
    for path in &ddl_migrations {
        let p = Path::new(path);
        if p.exists() {
            let schema = std::fs::read_to_string(p)?;
            sqlx::raw_sql(&schema).execute(pool).await?;
        }
    }

    // 阶段二：安全地添加新列，仅在列不存在时执行 ALTER TABLE
    // 格式为（表名, 列名, 数据类型, 默认值）
    let alter_migrations: &[(&str, &str, &str, &str)] = &[
        // (table, column, type, default)
        ("threads", "is_essence", "INTEGER", "0"),       // 帖子是否为精华帖
        ("users", "custom_title", "TEXT", "''"),         // 用户自定义头衔
        ("users", "epithet", "TEXT", "''"),              // 用户称号
        ("users", "epithet_color", "TEXT", "''"),        // 称号颜色
        ("users", "last_login_ip", "TEXT", "''"),        // 最后登录 IP
        ("users", "last_login_at", "TEXT", "''"),        // 最后登录时间
        ("users", "last_login_ua", "TEXT", "''"),        // 最后登录浏览器 User-Agent
        ("forums", "view_perm", "INTEGER", "0"),         // 浏览权限（0=所有人, 1=登录, 2=版主+, 3=管理员）
        ("forums", "post_perm", "INTEGER", "0"),         // 发帖权限（0=所有用户, 1=版主+, 2=管理员）
    ];
    for (table, column, col_type, default) in alter_migrations {
        if !column_exists(pool, table, column).await {
            let sql = format!(
                "ALTER TABLE {} ADD COLUMN {} {} NOT NULL DEFAULT {}",
                table, column, col_type, default
            );
            sqlx::raw_sql(&sql).execute(pool).await?;
        }
    }

    Ok(())
}

/// 扫描头像目录，修复数据库中缺失的头像记录
/// 场景：头像文件存在于 static/avatars/ 目录中，但数据库 avatar 字段为空
/// 遍历用户列表，检查是否存在对应的头像文件（支持 jpg/png/gif/webp 格式）
pub async fn repair_avatars(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    let avatar_dir = std::path::Path::new("static/avatars");
    if !avatar_dir.exists() {
        return Ok(());
    }

    // 查询所有 avatar 字段为空的用户
    let users: Vec<(i64, String)> = sqlx::query_as("SELECT id, avatar FROM users WHERE avatar = '' OR avatar IS NULL")
        .fetch_all(pool)
        .await
        .unwrap_or_default();

    // 支持的头像文件格式
    let extensions = ["jpg", "png", "gif", "webp"];
    for (user_id, _) in &users {
        // 按优先级检查各格式的头像文件是否存在
        for ext in &extensions {
            let path = avatar_dir.join(format!("{}.{}", user_id, ext));
            if path.exists() {
                let avatar_name = format!("{}.{}", user_id, ext);
                sqlx::query("UPDATE users SET avatar = ? WHERE id = ?")
                    .bind(&avatar_name)
                    .bind(user_id)
                    .execute(pool)
                    .await
                    .ok();
                tracing::info!("Repaired avatar for user {}: {}", user_id, avatar_name);
                break;
            }
        }
    }

    Ok(())
}

// 初始化站点默认设置
// 使用 INSERT OR IGNORE 确保只在设置项不存在时插入默认值，不会覆盖已有设置
pub async fn seed_settings(pool: &sqlx::SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    // 默认设置项列表：(键名, 默认值)
    let defaults = [
        ("site_name", "RustForum"),
        ("site_description", "A modern forum system built with Rust + Axum + SQLite"),
        ("site_keywords", "forum,rust,axum,sqlite"),
        ("site_footer_text", "Powered by RustForum"),
        ("allow_register", "1"),                        // 是否开放注册（1=是，0=否）
        ("invite_required", "0"),                       // 注册是否需要邀请码（1=需要，0=不需要）
        ("max_avatar_size", "524288"),                  // 头像大小上限（字节，默认 512KB）
        ("credits_checkin", "5"),                       // 签到奖励积分
        ("credits_thread", "3"),                        // 发帖奖励积分
        ("credits_reply", "2"),                         // 回复奖励积分
        ("credits_essence", "20"),                      // 精华帖奖励积分
        ("ai_review_enabled", "0"),                     // AI 审核开关（1=启用，0=禁用）
        ("ai_review_api_url", ""),                      // AI 审核 API 地址
        ("ai_review_api_key", ""),                      // AI 审核 API 密钥
        ("ai_review_model", "gpt-4o-mini"),             // AI 审核使用的模型
        ("ai_review_prompt", "你是一个内容安全审查助手。请审查以下用户发布的内容是否包含：\n1. 违法违规信息（政治敏感、暴力、色情等）\n2. 垃圾广告、恶意推广\n3. 人身攻击、侮辱歧视\n4. 虚假信息、诈骗内容\n\n请以JSON格式返回审查结果：\n{\"safe\": true/false, \"level\": \"safe/warning/danger\", \"reason\": \"具体说明\"}"),  // AI 审核提示词
        // 邮件服务设置
        ("email_enabled", "0"),                            // 邮件服务开关（1=启用，0=禁用）
        ("email_provider", "smtp"),                        // 发送方式（smtp / sendflare）
        ("email_from_name", ""),                           // 发件人名称
        ("email_from_address", ""),                        // 发件人地址
        ("email_sendflare_api_url", "https://api.sendflare.com"), // SendFlare API 地址
        ("email_sendflare_api_key", ""),                   // SendFlare API Key
        ("email_smtp_host", ""),                           // SMTP 服务器地址
        ("email_smtp_port", "465"),                        // SMTP 端口（465=TLS, 587=STARTTLS）
        ("email_smtp_username", ""),                       // SMTP 用户名
        ("email_smtp_password", ""),                       // SMTP 密码
        ("email_smtp_encryption", "tls"),                  // SMTP 加密方式（tls / starttls）
        ("email_verification_enabled", "0"),               // 注册邮箱验证开关（1=启用，0=禁用）
        ("email_verify_expire_hours", "24"),               // 邮箱验证链接有效时长（小时）
        ("site_url", "http://localhost:3000"),              // 站点 URL（用于生成验证链接等）
        ("setup_completed", "0"),                            // 安装向导是否完成
    ];
    for (key, value) in &defaults {
        sqlx::query("INSERT OR IGNORE INTO settings (key, value) VALUES (?, ?)")
            .bind(key)
            .bind(value)
            .execute(pool)
            .await?;
    }
    Ok(())
}
