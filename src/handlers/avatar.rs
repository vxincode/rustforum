// 头像处理器：处理头像上传（支持 multipart 表单）和头像删除
// 支持 JPG/PNG/GIF/WebP 格式，有文件大小限制，上传新头像前会自动清理旧头像文件

use axum::{
    extract::State,
    response::{Html, IntoResponse, Redirect},
};
use axum::extract::Multipart;

use crate::config::AppState;
use crate::middleware::auth::AuthUser;
use crate::templates::render_error;

// 允许上传的图片 MIME 类型
static ALLOWED_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

// 头像上传处理：验证文件类型和大小，保存文件到磁盘，更新数据库中的头像路径
pub async fn upload_avatar(
    AuthUser(user): AuthUser,
    state: State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let avatar_dir = &state.config.avatar_dir;
    let max_size = state.config.max_avatar_size;

    // 确保头像存储目录存在
    let _ = std::fs::create_dir_all(avatar_dir);

    while let Some(field) = multipart.next_field().await.ok().flatten() {
        // 验证文件类型是否为允许的图片格式
        let content_type = field.content_type().unwrap_or("").to_string();
        if !ALLOWED_TYPES.contains(&content_type.as_str()) {
            return Html(render_error("只支持 JPG/PNG/GIF/WebP 格式")).into_response();
        }

        // 读取上传的文件数据
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(_) => return Html(render_error("上传失败")).into_response(),
        };

        // 验证文件大小是否超过限制
        if data.len() as u64 > max_size {
            return Html(render_error("头像文件不能超过512KB")).into_response();
        }

        // 根据 MIME 类型确定文件扩展名
        let ext = match content_type.as_str() {
            "image/jpeg" => "jpg",
            "image/png" => "png",
            "image/gif" => "gif",
            "image/webp" => "webp",
            _ => "png",
        };

        // 删除用户旧的头像文件（所有格式）
        let _ = cleanup_avatar_files(avatar_dir, user.id);

        // 将新头像保存到磁盘，文件名格式为 {用户ID}.{扩展名}
        let filename = format!("{}/{}.{}", avatar_dir, user.id, ext);
        if std::fs::write(&filename, &data).is_err() {
            return Html(render_error("保存失败")).into_response();
        }

        // 更新数据库中的头像路径
        let avatar_path = format!("{}.{}", user.id, ext);
        sqlx::query("UPDATE users SET avatar = ? WHERE id = ?")
            .bind(&avatar_path)
            .bind(user.id)
            .execute(&state.pool)
            .await
            .ok();
    }

    Redirect::to("/profile/edit").into_response()
}

// 头像删除处理：清理磁盘上的头像文件，将数据库中头像字段置空
pub async fn delete_avatar(
    AuthUser(user): AuthUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let avatar_dir = &state.config.avatar_dir;
    // 删除所有格式的头像文件
    let _ = cleanup_avatar_files(avatar_dir, user.id);

    // 将用户头像字段设为空字符串
    sqlx::query("UPDATE users SET avatar = '' WHERE id = ?")
        .bind(user.id)
        .execute(&state.pool)
        .await
        .ok();

    Redirect::to("/profile/edit").into_response()
}

// 清理指定用户的所有头像文件（遍历所有支持的扩展名）
fn cleanup_avatar_files(avatar_dir: &str, user_id: i64) -> std::io::Result<()> {
    for ext in &["jpg", "png", "gif", "webp"] {
        let path = format!("{}/{}.{}", avatar_dir, user_id, ext);
        if std::path::Path::new(&path).exists() {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}
