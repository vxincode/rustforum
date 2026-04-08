// 配置管理模块
// 职责：从环境变量读取应用配置，定义全局共享状态 AppState
// 配置项包括数据库连接地址、监听端口、分页参数、头像上传限制等

use std::env;
use sqlx::SqlitePool;
pub use redis::aio::ConnectionManager;

// 应用配置结构体，存储从环境变量读取的各项配置
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,       // Database connection string, e.g. "sqlite:forum.db?mode=rwc"
    pub listen_addr: String,        // HTTP 监听地址，如 "0.0.0.0:3000"
    pub threads_per_page: i64,      // 每页显示的主题帖数量
    pub posts_per_page: i64,        // 每页显示的回复数量
    pub avatar_dir: String,         // 头像文件存储目录
    pub max_avatar_size: u64,       // 头像文件大小上限（字节）
    // 以下字段保留供将来使用，目前站点信息已迁移到数据库 settings 表
    #[allow(dead_code)]
    pub site_name: String,          // 站点名称（已弃用，改用 site_config 模块）
    #[allow(dead_code)]
    pub site_description: String,   // 站点描述（已弃用，改用 site_config 模块）
    #[allow(dead_code)]
    pub session_secret: String,     // 会话加密密钥
    pub redis_url: String,          // Redis 连接地址
}

impl Config {
    // 从环境变量加载配置，未设置时使用默认值
    pub fn from_env() -> Self {
        Config {
            // Database connection string, defaults to forum.db in current directory, rwc mode creates if not exists
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:forum.db?mode=rwc".to_string()),
            // 监听地址，默认绑定所有网卡的 3000 端口
            listen_addr: env::var("LISTEN_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            // 站点名称（已迁移到数据库管理）
            site_name: env::var("SITE_NAME")
                .unwrap_or_else(|_| "RustForum".to_string()),
            // 站点描述（已迁移到数据库管理）
            site_description: env::var("SITE_DESC")
                .unwrap_or_else(|_| "A modern forum system built with Rust + Axum + SQLite".to_string()),
            // 每页主题帖数，默认 30 条
            threads_per_page: env::var("THREADS_PER_PAGE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(30),
            // 每页回复数，默认 20 条
            posts_per_page: env::var("POSTS_PER_PAGE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(20),
            // 会话加密密钥，生产环境应通过环境变量设置
            session_secret: env::var("SESSION_SECRET")
                .unwrap_or_else(|_| "rustforum-secret-change-me".to_string()),
            // 头像文件存储目录
            avatar_dir: env::var("AVATAR_DIR")
                .unwrap_or_else(|_| "static/avatars".to_string()),
            // 头像文件大小上限，默认 512KB（524288 字节）
            max_avatar_size: env::var("MAX_AVATAR_SIZE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(524288),
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
        }
    }
}

// Axum 应用全局共享状态
// 包含数据库连接池和配置信息，通过 with_state 注入到路由中
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,   // SQLite 数据库连接池
    pub config: Config,     // 应用配置
    pub redis: Option<ConnectionManager>,  // Redis 连接管理器（可选，降级为无缓存模式）
}
