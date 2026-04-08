// AI 共享数据模型

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct AiShare {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub content: String,
    pub category: String,
    pub share_type: String,
    pub price: i64,
    pub download_count: i64,
    pub status: i64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct AiSharePurchase {
    pub id: i64,
    pub share_id: i64,
    pub user_id: i64,
    pub credits_paid: i64,
    pub created_at: String,
}

/// 列表页用：含作者信息
#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub struct AiShareList {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub category: String,
    pub share_type: String,
    pub price: i64,
    pub download_count: i64,
    pub status: i64,
    pub created_at: String,
    pub username: String,
    pub avatar: String,
}
