// 程序入口文件
// 职责：初始化日志、加载配置、连接数据库、执行迁移和种子数据、注册所有路由、启动 HTTP 服务器
// 本项目基于 Axum Web 框架，使用 SQLite 数据库，采用服务端渲染（SSR）方式生成页面

// 声明各子模块
mod cache;         // Redis 缓存辅助模块
mod config;        // 配置管理模块
mod db;            // 数据库初始化与迁移模块
mod email;         // 邮件发送模块（SendFlare API / SMTP）
mod handlers;      // 请求处理器模块（按功能分子模块）
mod middleware;     // 中间件模块（认证、限流、CSRF 防护）
mod models;        // 数据模型模块
mod site_config;   // 全局站点设置缓存模块
mod templates;     // HTML 模板渲染模块

use axum::{routing::{get, post}, Router};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use config::{AppState, Config};

// 程序主入口，使用 tokio 异步运行时
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志系统，默认级别为 info，可通过环境变量 RUST_LOG 调整
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("rustforum=info".parse()?))
        .init();

    // 从环境变量加载应用配置
    let config = Config::from_env();
    let listen_addr = config.listen_addr.clone();

    // 初始化数据库连接池并执行启动流程
    let pool = db::init_pool(&config.database_url).await?;  // 创建 SQLite 连接池
    db::run_migrations(&pool).await?;   // 执行数据库迁移（建表、加字段）
    db::seed_settings(&pool).await?;    // 初始化站点默认设置
    db::repair_avatars(&pool).await?;   // 修复孤立的头像文件记录

    tracing::info!("Database initialized");

    // 将数据库中的站点设置加载到全局缓存中
    site_config::load_from_db(&pool).await;

    // 初始化 Redis 连接（失败则降级为无缓存模式）
    let redis = match redis::Client::open(config.redis_url.as_str()) {
        Ok(client) => match tokio::time::timeout(
            std::time::Duration::from_secs(3),
            client.get_connection_manager(),
        ).await {
            Ok(Ok(cm)) => {
                tracing::info!("Redis connected");
                Some(cm)
            }
            Ok(Err(e)) => {
                tracing::warn!("Redis connection failed, running without cache: {}", e);
                None
            }
            Err(_) => {
                tracing::warn!("Redis connection timed out, running without cache");
                None
            }
        },
        Err(e) => {
            tracing::warn!("Redis client creation failed: {}", e);
            None
        }
    };

    // 构建应用共享状态，包含数据库连接池、配置和 Redis 连接
    let state = AppState {
        pool,
        config,
        redis,
    };

    // 注册所有 HTTP 路由
    let app = Router::new()
        // ========== 安装向导路由 ==========
        .route("/setup", get(handlers::setup::setup_page).post(handlers::setup::setup_submit))
        // ========== 页面路由（服务端渲染 SSR） ==========
        .route("/", get(handlers::index::index))                          // 首页
        .route("/auth/login", get(handlers::auth::login_page))           // 登录页面
        .route("/auth/login", post(handlers::auth::login))               // 登录提交
        .route("/auth/register", get(handlers::auth::register_page))     // 注册页面
        .route("/auth/register", post(handlers::auth::register))         // 注册提交
        .route("/auth/logout", get(handlers::auth::logout))              // 退出登录
        .route("/auth/verify", get(handlers::auth::verify_email))        // 邮箱验证
        .route("/auth/resend-verify", get(handlers::auth::resend_verify)) // 重发验证邮件
        .route("/forums", get(handlers::forum::forum_list))              // 版块列表
        .route("/about", get(handlers::about::about_page))               // 关于页面
        .route("/terms", get(handlers::about::terms_page))               // 服务条款
        .route("/privacy", get(handlers::about::privacy_page))           // 隐私政策
        .route("/contact", get(handlers::about::contact_page))           // 联系我们
        .route("/forum/{forum_id}", get(handlers::forum::forum_view))   // 版块详情（帖子列表）
        .route("/new", get(handlers::forum::new_thread_generic))         // 通用发帖入口
        .route("/forum/{forum_id}/new", get(handlers::forum::new_thread_page))   // 发帖页面
        .route("/forum/{forum_id}/new", post(handlers::forum::create_thread))    // 发帖提交
        .route("/thread/{thread_id}", get(handlers::thread::thread_view))        // 帖子详情页
        .route("/thread/{thread_id}/reply", post(handlers::thread::reply_thread)) // 回复帖子
        .route("/thread/{thread_id}/edit", get(handlers::thread::edit_thread_page))  // 编辑帖子页面
        .route("/thread/{thread_id}/edit", post(handlers::thread::edit_thread))      // 编辑帖子提交
        .route("/thread/{thread_id}/delete", post(handlers::thread::delete_thread))  // 删除帖子
        .route("/post/{post_id}/edit", get(handlers::thread::edit_post_page))   // 编辑回复页面
        .route("/post/{post_id}/edit", post(handlers::thread::edit_post))       // 编辑回复提交
        .route("/post/{post_id}/delete", post(handlers::thread::delete_post))   // 删除回复
        .route("/profile", get(handlers::profile::profile_page))         // 个人资料页
        .route("/profile/edit", get(handlers::profile::profile_edit_page))   // 编辑资料页面
        .route("/profile/edit", post(handlers::profile::profile_edit))       // 编辑资料提交
        .route("/profile/password", post(handlers::profile::change_password)) // 修改密码
        .route("/profile/verify-email", get(handlers::profile::verify_email_page).post(handlers::profile::verify_email_code)) // 邮箱验证码
        .route("/profile/avatar", post(handlers::avatar::upload_avatar))     // 上传头像
        .route("/profile/avatar/delete", post(handlers::avatar::delete_avatar)) // 删除头像
        .route("/user/{user_id}", get(handlers::profile::user_profile))  // 查看他人资料
        .route("/thread/mine", get(handlers::profile::my_threads))       // 我的帖子
        // ========== 私信功能路由 ==========
        .route("/messages", get(handlers::message::inbox))               // 收件箱
        .route("/messages/compose", get(handlers::message::compose_page)) // 写私信页面
        .route("/messages/send", post(handlers::message::send_message))  // 发送私信
        .route("/messages/{user_id}", get(handlers::message::conversation))        // 与某人的对话
        .route("/messages/{user_id}/reply", post(handlers::message::reply_message)) // 回复私信
        .route("/messages/{user_id}/delete", post(handlers::message::delete_conversation)) // 删除对话
        .route("/api/messages/unread", get(handlers::message::api_unread_count))   // 未读消息数接口
        // ========== 管理后台路由 ==========
        .route("/admin", get(handlers::admin::dashboard))                // 管理面板首页
        .route("/admin/forums", get(handlers::admin::forums))            // 版块管理
        .route("/admin/forums/create", post(handlers::admin::create_forum))       // 创建版块
        .route("/admin/forums/{forum_id}/edit", post(handlers::admin::edit_forum)) // 编辑版块
        .route("/admin/forums/{forum_id}/delete", get(handlers::admin::delete_forum)) // 删除版块
        .route("/admin/forums/{forum_id}/moderators/add", post(handlers::admin::add_forum_moderator))   // 添加版主
        .route("/admin/forums/{forum_id}/moderators/{user_id}/remove", post(handlers::admin::remove_forum_moderator)) // 移除版主
        .route("/admin/users", get(handlers::admin::users))              // 用户管理
        .route("/admin/users/{user_id}/toggle", get(handlers::admin::toggle_user_status)) // 切换用户状态（启用/禁用）
        .route("/admin/users/{user_id}/group/{group_id}", get(handlers::admin::set_user_group)) // 设置用户组
        .route("/admin/users/{user_id}/mute", post(handlers::admin::mute_user))    // 禁言用户
        .route("/admin/users/{user_id}/unmute", get(handlers::admin::unmute_user)) // 解除禁言
        .route("/admin/login-logs", get(handlers::admin::login_logs_page)) // 登录日志
        .route("/admin/threads", get(handlers::admin::threads_page))     // 帖子管理
        .route("/admin/thread/{thread_id}/sticky", post(handlers::admin::toggle_sticky))   // 置顶/取消置顶
        .route("/admin/thread/{thread_id}/essence", post(handlers::admin::toggle_essence)) // 加精/取消加精
        .route("/admin/thread/{thread_id}/close", post(handlers::admin::toggle_close))     // 关闭/开启帖子
        .route("/admin/thread/{thread_id}/move", get(handlers::admin::move_thread_page).post(handlers::admin::move_thread)) // 移动帖子
        .route("/admin/thread/{thread_id}/delete", post(handlers::admin::admin_delete_thread)) // 管理员删帖
        .route("/admin/post/{post_id}/delete", post(handlers::admin::admin_delete_post))   // 管理员删回复
        .route("/admin/reports", get(handlers::admin::reports_page))     // 举报管理
        .route("/admin/reports/{report_id}/action", post(handlers::admin::report_action))  // 处理举报
        .route("/admin/blacklist", get(handlers::admin::blacklist_page)) // 黑名单管理
        .route("/admin/blacklist/add", post(handlers::admin::add_blacklist))         // 添加黑名单
        .route("/admin/blacklist/{entry_id}/delete", post(handlers::admin::remove_blacklist)) // 移除黑名单
        .route("/admin/invite-codes", get(handlers::admin::invite_codes_page))       // 邀请码管理
        .route("/admin/invite-codes/create", post(handlers::admin::create_invite_codes)) // 生成邀请码
        .route("/admin/invite-codes/{id}/delete", post(handlers::admin::delete_invite_code)) // 删除邀请码
        .route("/admin/review", get(handlers::admin::review_page))       // 内容审核页面
        .route("/admin/review/check", post(handlers::admin::review_content)) // 内容审核提交
        .route("/admin/settings", get(handlers::admin::settings_page))   // 系统设置首页
        .route("/admin/settings/site", get(handlers::admin::settings_site_page).post(handlers::admin::settings_site_save))       // 站点设置
        .route("/admin/settings/register", get(handlers::admin::settings_register_page).post(handlers::admin::settings_register_save)) // 注册设置
        .route("/admin/settings/credits", get(handlers::admin::settings_credits_page).post(handlers::admin::settings_credits_save))     // 积分设置
        .route("/admin/settings/upload", get(handlers::admin::settings_upload_page).post(handlers::admin::settings_upload_save))       // 上传设置
        .route("/admin/settings/ai", get(handlers::admin::settings_ai_page).post(handlers::admin::settings_ai_save))                   // AI 审核设置
        .route("/admin/settings/email", get(handlers::admin::settings_email_page).post(handlers::admin::settings_email_save))           // 邮件设置
        .route("/admin/settings/email/test", post(handlers::admin::settings_email_test))                                                // 测试邮件发送
        // ========== 备份与恢复路由 ==========
        .route("/admin/backup", get(handlers::backup::backup_page))                          // 备份管理页面
        .route("/admin/backup/create", post(handlers::backup::create_backup))                // 创建备份
        .route("/admin/backup/download/{filename}", get(handlers::backup::download_backup))  // 下载备份文件
        .route("/admin/backup/restore", post(handlers::backup::restore_backup))              // 恢复备份
        .route("/admin/backup/delete/{filename}", post(handlers::backup::delete_backup))     // 删除备份文件
        // ========== JSON API 接口 ==========
        .route("/api/threads", get(handlers::api::api_threads))          // 帖子列表接口
        .route("/api/forums", get(handlers::api::api_forums))            // 版块列表接口
        .route("/api/stats", get(handlers::api::api_stats))              // 站点统计接口
        .route("/api/search", get(handlers::api::api_search))            // 搜索接口
        .route("/api/auth/login", post(handlers::api::api_login))        // API 登录
        .route("/api/auth/register", post(handlers::api::api_register))  // API 注册
        .route("/api/auth/logout", get(handlers::api::api_logout))       // API 退出
        .route("/api/me", get(handlers::api::api_me))                    // 获取当前用户信息
        .route("/api/thread/{forum_id}/new", post(handlers::api::api_new_thread)) // API 发帖
        .route("/api/thread/{thread_id}/reply", post(handlers::api::api_reply))   // API 回复
        .route("/api/user/{user_id}/card", get(handlers::api::api_user_card))     // 用户卡片数据（悬浮卡）
        // ========== 通知 API ==========
        .route("/api/notifications", get(handlers::notification::api_notifications))              // 获取通知列表
        .route("/api/notifications/{notif_id}/read", post(handlers::notification::api_notification_read))   // 标记单条已读
        .route("/api/notifications/read-all", post(handlers::notification::api_notification_read_all))       // 全部标记已读
        .route("/api/post/{post_id}", get(handlers::notification::api_get_post))                   // 获取帖子内容（行内编辑用）
        .route("/api/post/{post_id}/edit", post(handlers::notification::api_edit_post))            // 行内编辑回复
        .route("/api/thread/{thread_id}/edit", post(handlers::notification::api_edit_thread))      // 行内编辑主题帖
        // ========== 签到与排行榜 ==========
        .route("/api/checkin", post(handlers::checkin::api_checkin))         // 每日签到
        .route("/api/checkin/status", get(handlers::checkin::api_checkin_status)) // 签到状态
        .route("/api/leaderboard", get(handlers::checkin::api_leaderboard))  // 积分排行榜
        .route("/api/users/recent", get(handlers::checkin::api_new_users))   // 最新注册用户
        .route("/api/links", get(handlers::checkin::api_friendly_links))     // 友情链接接口
        .route("/api/report", post(handlers::report::api_report))            // 举报提交接口
        // ========== AI 共享模块路由 ==========
        .route("/ai", get(handlers::ai_share::ai_share_list))                       // AI 共享列表
        .route("/ai/create", get(handlers::ai_share::ai_share_create_page)           // 创建页面
            .post(handlers::ai_share::ai_share_create))                              // 提交创建
        .route("/ai/{id}", get(handlers::ai_share::ai_share_detail))                 // 详情页
        .route("/ai/{id}/edit", get(handlers::ai_share::ai_share_edit_page)          // 编辑页面
            .post(handlers::ai_share::ai_share_edit))                                // 提交编辑
        .route("/ai/{id}/delete", post(handlers::ai_share::ai_share_delete))         // 删除
        .route("/ai/{id}/purchase", post(handlers::ai_share::ai_share_purchase))     // 积分兑换
        // ========== 静态文件服务 ==========
        .nest_service("/static", ServeDir::new("static"))  // 托管 static/ 目录下的静态资源
        .layer(axum::middleware::from_fn_with_state(state.clone(), middleware::setup_guard::check_setup))
        .layer(TraceLayer::new_for_http())  // 请求日志中间件
        .with_state(state);  // 注入应用共享状态

    // 绑定 TCP 监听地址并启动 HTTP 服务器
    tracing::info!("Listening on {}", listen_addr);
    let listener = tokio::net::TcpListener::bind(&listen_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
