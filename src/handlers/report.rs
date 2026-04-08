// 举报处理器：处理用户举报提交，支持对帖子、回复、用户三种类型的举报
// 举报记录存入数据库供管理员在后台查看和处理

use axum::{
    extract::State,
    response::{IntoResponse, Json},
    Form,
};
use serde::{Deserialize, Serialize};

use crate::config::AppState;
use crate::middleware::auth::AuthUser;

// 举报表单数据：包含举报目标类型、目标 ID、举报原因和可选的详细描述
#[derive(Deserialize)]
pub struct ReportForm {
    pub target_type: String,
    pub target_id: i64,
    pub reason: String,
    pub description: Option<String>,
}

// 举报 API 响应结构
#[derive(Serialize)]
struct ReportResponse {
    ok: bool,
    msg: String,
}

// 举报提交 API：需登录，验证举报类型和原因后写入数据库
pub async fn api_report(
    _user: AuthUser,
    state: State<AppState>,
    Form(form): Form<ReportForm>,
) -> impl IntoResponse {
    if form.reason.trim().is_empty() {
        return Json(ReportResponse { ok: false, msg: "请填写举报原因".to_string() });
    }

    // 验证举报类型必须是 thread/post/user 之一
    if !["thread", "post", "user"].contains(&form.target_type.as_str()) {
        return Json(ReportResponse { ok: false, msg: "无效的举报类型".to_string() });
    }

    sqlx::query(
        "INSERT INTO reports (reporter_id, target_type, target_id, reason, description) VALUES (?, ?, ?, ?, ?)"
    )
    .bind(_user.0.id)
    .bind(&form.target_type)
    .bind(form.target_id)
    .bind(form.reason.trim())
    .bind(form.description.unwrap_or_default())
    .execute(&state.pool)
    .await
    .ok();

    Json(ReportResponse { ok: true, msg: "举报已提交，我们会尽快处理".to_string() })
}
