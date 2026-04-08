// 静态页面处理器：关于我们、服务条款、隐私政策、联系方式
// 这些页面内容通过模板渲染，无需数据库查询

use axum::response::{Html, IntoResponse};

use crate::templates::{render_about, render_contact, render_privacy, render_terms};

// 关于我们页面
pub async fn about_page() -> impl IntoResponse {
    Html(render_about())
}

// 服务条款页面
pub async fn terms_page() -> impl IntoResponse {
    Html(render_terms())
}

// 隐私政策页面
pub async fn privacy_page() -> impl IntoResponse {
    Html(render_privacy())
}

// 联系方式页面
pub async fn contact_page() -> impl IntoResponse {
    Html(render_contact())
}
