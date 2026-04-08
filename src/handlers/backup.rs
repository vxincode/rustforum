// 数据库备份与恢复处理器：支持创建 ZIP 备份（含数据库和头像）、下载备份、
// 上传恢复备份（恢复前自动创建安全备份）、删除备份文件
// 备份格式为 ZIP，包含 database.db、manifest.json 和 avatars/ 目录
// 所有操作仅管理员可执行，文件名使用严格的格式验证防止路径遍历攻击

use axum::{
    extract::{Multipart, Path, State},
    response::{Html, IntoResponse, Redirect},
};
use std::fs;
use std::io::{Read as IoRead, Write};
use zip::write::SimpleFileOptions;

use crate::config::AppState;
use crate::middleware::auth::AdminUser;
use crate::templates::{admin_layout, render_admin_backup, render_error};

// 恢复前自动备份的文件名前缀
const PRE_RESTORE_PREFIX: &str = "backup_pre_restore_";

// 验证备份文件名格式：必须为 backup_YYYYMMDD_HHMMSS.zip
fn is_valid_backup_name(name: &str) -> bool {
    if !name.starts_with("backup_") || !name.ends_with(".zip") {
        return false;
    }
    let core = &name[7..name.len() - 4]; // strip "backup_" and ".zip"
    // core should be YYYYMMDD_HHMMSS
    if core.len() != 15 {
        return false;
    }
    let mut chars = core.chars();
    // YYYYMMDD_HHMMSS: 8 digits, underscore, 6 digits
    for _ in 0..8 {
        if !chars.next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            return false;
        }
    }
    if chars.next() != Some('_') {
        return false;
    }
    for _ in 0..6 {
        if !chars.next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            return false;
        }
    }
    // Must not contain path separators
    !name.contains('/') && !name.contains('\\') && !name.contains("..")
}

// 获取备份文件存储目录路径
fn backups_dir() -> String {
    "backups".to_string()
}

// 确保备份目录存在，不存在则递归创建
fn ensure_backups_dir() -> std::io::Result<()> {
    fs::create_dir_all(backups_dir())
}

// 生成当前时间戳字符串，格式为 YYYYMMDD_HHMMSS
fn now_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

// 从数据库连接 URL 中提取 SQLite 文件路径
fn db_path_from_url(url: &str) -> String {
    let without_prefix = url.strip_prefix("sqlite:").unwrap_or(url);
    let path = without_prefix.split('?').next().unwrap_or(without_prefix);
    path.to_string()
}

// 备份文件信息结构
pub struct BackupInfo {
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: String,
}

// 列出备份目录中的所有备份文件，按文件名倒序排列（最新的在前）
fn list_backups() -> Vec<BackupInfo> {
    let dir = backups_dir();
    let mut backups: Vec<BackupInfo> = Vec::new();

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !is_valid_backup_name(&name) && !name.starts_with(PRE_RESTORE_PREFIX) {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                let created = meta
                    .created()
                    .ok()
                    .map(|t| {
                        let dt: chrono::DateTime<chrono::Local> = t.into();
                        dt.format("%Y-%m-%d %H:%M:%S").to_string()
                    })
                    .unwrap_or_else(|| "-".to_string());
                backups.push(BackupInfo {
                    filename: name,
                    size_bytes: meta.len(),
                    created_at: created,
                });
            }
        }
    }

    backups.sort_by(|a, b| b.filename.cmp(&a.filename));
    backups
}

// 格式化文件大小为人类可读形式（B/KB/MB）
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// === 处理器 ===

// 备份管理页面：列出所有备份文件，展示文件名、大小和创建时间
pub async fn backup_page(_admin: AdminUser) -> impl IntoResponse {
    let backups = tokio::task::spawn_blocking(|| list_backups()).await.unwrap_or_default();
    let items: Vec<(String, String, String, String)> = backups
        .into_iter()
        .map(|b| {
            (
                b.filename.clone(),
                format_size(b.size_bytes),
                b.created_at,
                b.filename,
            )
        })
        .collect();
    Html(render_admin_backup(&items))
}

// 创建备份：先执行 WAL 检查点，再在阻塞线程中执行备份（压缩数据库和头像文件）
pub async fn create_backup(
    _admin: AdminUser,
    state: State<AppState>,
) -> impl IntoResponse {
    let db_url = state.config.database_url.clone();
    let avatar_dir = state.config.avatar_dir.clone();
    let pool = state.pool.clone();

    // WAL 检查点：将 WAL 日志合并到主数据库，确保备份完整性
    if let Err(e) = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await
    {
        tracing::warn!("WAL checkpoint failed: {}", e);
    }

    // 在阻塞线程中执行备份操作（避免阻塞异步运行时）
    let result = tokio::task::spawn_blocking(move || create_backup_sync(&db_url, &avatar_dir)).await;

    match result {
        Ok(Ok(_)) => Redirect::to("/admin/backup").into_response(),
        Ok(Err(e)) => {
            tracing::error!("Backup failed: {}", e);
            Html(render_error(&format!("备份失败: {}", e))).into_response()
        }
        Err(e) => {
            tracing::error!("Backup task panicked: {}", e);
            Html(render_error("备份任务异常")).into_response()
        }
    }
}

// 同步创建备份：将数据库文件和头像目录打包为 ZIP 文件
// ZIP 包含 database.db（数据库文件）、manifest.json（元数据）和 avatars/（头像目录）
fn create_backup_sync(db_url: &str, avatar_dir: &str) -> Result<(), String> {
    ensure_backups_dir().map_err(|e| format!("创建备份目录失败: {}", e))?;

    let db_path = db_path_from_url(db_url);
    let timestamp = now_timestamp();
    let zip_filename = format!("backup_{}.zip", timestamp);
    let zip_path = format!("{}/{}", backups_dir(), zip_filename);

    // 先将数据库文件复制到临时位置，避免备份过程中数据库被修改
    let tmp_db = format!("{}/tmp_backup_{}.db", backups_dir(), timestamp);
    fs::copy(&db_path, &tmp_db).map_err(|e| format!("复制数据库失败: {}", e))?;

    let db_data = fs::read(&tmp_db).map_err(|e| format!("读取数据库副本失败: {}", e))?;
    let _ = fs::remove_file(&tmp_db);

    // 统计头像文件数量（写入 manifest）
    let avatar_count = fs::read_dir(avatar_dir).map(|d| d.count()).unwrap_or(0);
    let db_size = db_data.len();

    // 构建 manifest 元数据文件
    let manifest = serde_json::json!({
        "version": "1.0",
        "timestamp": timestamp,
        "db_size": db_size,
        "avatar_count": avatar_count,
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap_or_default();

    // 创建 ZIP 文件并逐项添加内容
    let file = fs::File::create(&zip_path).map_err(|e| format!("创建ZIP文件失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Add database.db
    zip.start_file("database.db", options)
        .map_err(|e| format!("写入数据库到ZIP失败: {}", e))?;
    zip.write_all(&db_data)
        .map_err(|e| format!("写入数据库数据失败: {}", e))?;

    // Add manifest.json
    zip.start_file("manifest.json", options)
        .map_err(|e| format!("写入manifest失败: {}", e))?;
    zip.write_all(&manifest_bytes)
        .map_err(|e| format!("写入manifest数据失败: {}", e))?;

    // 添加头像目录中的所有文件
    if fs::metadata(avatar_dir).is_ok() {
        let entries = fs::read_dir(avatar_dir).map_err(|e| format!("读取头像目录失败: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let zip_name = format!("avatars/{}", name);
            let data = fs::read(&path).unwrap_or_default();

            zip.start_file(&zip_name, options)
                .map_err(|e| format!("添加头像到ZIP失败: {}", e))?;
            zip.write_all(&data)
                .map_err(|e| format!("写入头像数据失败: {}", e))?;
        }
    }

    zip.finish()
        .map_err(|e| format!("完成ZIP写入失败: {}", e))?;

    tracing::info!("Backup created: {}", zip_filename);
    Ok(())
}

// 下载备份文件：验证文件名安全性后返回 ZIP 文件内容
pub async fn download_backup(
    _admin: AdminUser,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    // 允许下载常规备份和恢复前的安全备份
    if !is_valid_backup_name(&filename) && !filename.starts_with(PRE_RESTORE_PREFIX) {
        return Html(render_error("无效的备份文件名")).into_response();
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Html(render_error("无效的备份文件名")).into_response();
    }

    let path = format!("{}/{}", backups_dir(), filename);
    let data = match tokio::task::spawn_blocking(move || fs::read(&path)).await {
        Ok(Ok(d)) => d,
        _ => return Html(render_error("备份文件不存在")).into_response(),
    };

    let disposition = format!("attachment; filename=\"{}\"", filename);
    (
        [
            ("content-type", "application/zip".to_string()),
            ("content-disposition", disposition),
        ],
        data,
    )
        .into_response()
}

// 恢复备份：上传 ZIP 文件，验证内容后恢复数据库和头像，恢复前自动创建安全备份
pub async fn restore_backup(
    _admin: AdminUser,
    state: State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let db_url = state.config.database_url.clone();
    let avatar_dir = state.config.avatar_dir.clone();
    let pool = state.pool.clone();

    let mut zip_data: Option<Vec<u8>> = None;

    while let Some(field) = multipart.next_field().await.ok().flatten() {
        let data = match field.bytes().await {
            Ok(d) => d,
            Err(_) => return Html(render_error("读取上传文件失败")).into_response(),
        };
        // 上传文件大小限制 100MB
        if data.len() > 100 * 1024 * 1024 {
            return Html(render_error("文件大小不能超过100MB")).into_response();
        }
        zip_data = Some(data.to_vec());
    }

    let zip_bytes = match zip_data {
        Some(d) => d,
        None => return Html(render_error("未选择备份文件")).into_response(),
    };

    // 恢复前执行 WAL 检查点
    if let Err(e) = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE)")
        .execute(&pool)
        .await
    {
        tracing::warn!("Pre-restore WAL checkpoint failed: {}", e);
    }

    let result =
        tokio::task::spawn_blocking(move || restore_backup_sync(&zip_bytes, &db_url, &avatar_dir))
            .await;

    match result {
        Ok(Ok(_)) => {
            let msg = "数据已成功恢复。请重启服务器以使新数据库生效。";
            let content = format!(
                r#"
<div class="space-y-6">
  <div class="bg-green-50 border border-green-200 rounded-lg p-6 text-center">
    <i class="fa fa-check-circle text-4xl text-green-500 mb-4"></i>
    <h2 class="text-xl font-bold text-green-700 mb-2">恢复完成</h2>
    <p class="text-green-600 mb-6">{msg}</p>
    <a href="/admin/backup" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors inline-block">返回备份管理</a>
  </div>
</div>"#
            );
            Html(admin_layout("恢复完成", "backup", &content)).into_response()
        }
        Ok(Err(e)) => {
            tracing::error!("Restore failed: {}", e);
            Html(render_error(&format!("恢复失败: {}", e))).into_response()
        }
        Err(e) => {
            tracing::error!("Restore task panicked: {}", e);
            Html(render_error("恢复任务异常")).into_response()
        }
    }
}

// 同步恢复备份：验证 ZIP 内容（包含数据库和 manifest）、验证 SQLite 文件头、
// 创建恢复前安全备份、提取数据库文件和头像文件
fn restore_backup_sync(zip_bytes: &[u8], db_url: &str, avatar_dir: &str) -> Result<(), String> {
    // 首先验证 ZIP 文件内容是否完整
    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("无效的ZIP文件: {}", e))?;

    // 检查必需文件是否存在
    let has_db = archive.by_name("database.db").is_ok();
    let has_manifest = archive.by_name("manifest.json").is_ok();

    if !has_db {
        return Err("备份文件缺少 database.db".to_string());
    }
    if !has_manifest {
        return Err("备份文件缺少 manifest.json".to_string());
    }

    // 验证 SQLite 文件头，确保数据库文件有效
    {
        let mut db_file = archive
            .by_name("database.db")
            .map_err(|e| format!("读取 database.db 失败: {}", e))?;
        let mut header = [0u8; 16];
        if db_file.read_exact(&mut header).is_err() {
            return Err("无法读取数据库文件头".to_string());
        }
        if &header != b"SQLite format 3\0" {
            return Err("数据库文件头验证失败，不是有效的SQLite文件".to_string());
        }
    }

    // 恢复前自动创建安全备份（防止恢复失败导致数据丢失）
    ensure_backups_dir().map_err(|e| format!("创建备份目录失败: {}", e))?;
    let pre_timestamp = now_timestamp();
    let pre_backup_name = format!("{}{}.zip", PRE_RESTORE_PREFIX, pre_timestamp);

    if let Err(e) = create_pre_restore_backup(db_url, avatar_dir, &pre_backup_name) {
        tracing::warn!("Pre-restore backup failed: {}", e);
    }

    // 重新打开 ZIP 归档（之前的验证消耗了迭代器）
    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("重新打开ZIP失败: {}", e))?;

    // 提取并覆盖数据库文件
    let db_path = db_path_from_url(db_url);
    {
        let mut db_file = archive
            .by_name("database.db")
            .map_err(|e| format!("读取 database.db 失败: {}", e))?;
        let mut db_data = Vec::new();
        db_file
            .read_to_end(&mut db_data)
            .map_err(|e| format!("读取数据库内容失败: {}", e))?;
        fs::write(&db_path, &db_data).map_err(|e| format!("写入数据库文件失败: {}", e))?;
    }

    // 清空并恢复头像目录
    let _ = fs::create_dir_all(avatar_dir);
    if fs::metadata(avatar_dir).is_ok() {
        if let Ok(entries) = fs::read_dir(avatar_dir) {
            for entry in entries.flatten() {
                let _ = fs::remove_file(entry.path());
            }
        }
    }

    // 再次打开 ZIP 归档用于提取头像文件
    let reader = std::io::Cursor::new(zip_bytes);
    let mut archive =
        zip::ZipArchive::new(reader).map_err(|e| format!("重新打开ZIP失败: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("读取ZIP条目失败: {}", e))?;
        let name = file.name().to_string();

        if name.starts_with("avatars/") && !file.is_dir() {
            let avatar_name = name.strip_prefix("avatars/").unwrap_or(&name);
            if avatar_name.is_empty() {
                continue;
            }
            let dest = format!("{}/{}", avatar_dir, avatar_name);
            let mut data = Vec::new();
            file.read_to_end(&mut data)
                .map_err(|e| format!("读取头像数据失败: {}", e))?;
            fs::write(&dest, &data)
                .map_err(|e| format!("写入头像文件失败: {}", e))?;
        }
    }

    tracing::info!("Restore completed from backup");
    Ok(())
}

// 创建恢复前的安全备份：在执行恢复操作前保存当前状态的快照
fn create_pre_restore_backup(
    db_url: &str,
    avatar_dir: &str,
    backup_name: &str,
) -> Result<(), String> {
    let zip_path = format!("{}/{}", backups_dir(), backup_name);

    let db_path = db_path_from_url(db_url);
    let db_data = fs::read(&db_path).map_err(|e| format!("读取当前数据库失败: {}", e))?;

    let avatar_count = fs::read_dir(avatar_dir).map(|d| d.count()).unwrap_or(0);
    let db_size = db_data.len();

    let manifest = serde_json::json!({
        "version": "1.0",
        "timestamp": "pre_restore",
        "db_size": db_size,
        "avatar_count": avatar_count,
    });
    let manifest_bytes = serde_json::to_vec_pretty(&manifest).unwrap_or_default();

    let file =
        fs::File::create(&zip_path).map_err(|e| format!("创建安全备份文件失败: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    zip.start_file("database.db", options)
        .map_err(|e| format!("写入数据库失败: {}", e))?;
    zip.write_all(&db_data)
        .map_err(|e| format!("写入数据失败: {}", e))?;

    zip.start_file("manifest.json", options)
        .map_err(|e| format!("写入manifest失败: {}", e))?;
    zip.write_all(&manifest_bytes)
        .map_err(|e| format!("写入manifest数据失败: {}", e))?;

    if fs::metadata(avatar_dir).is_ok() {
        let entries = fs::read_dir(avatar_dir).map_err(|e| format!("读取头像目录失败: {}", e))?;
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let zip_name = format!("avatars/{}", name);
            let data = fs::read(&path).unwrap_or_default();
            zip.start_file(&zip_name, options)
                .map_err(|e| format!("添加头像失败: {}", e))?;
            zip.write_all(&data)
                .map_err(|e| format!("写入头像数据失败: {}", e))?;
        }
    }

    zip.finish()
        .map_err(|e| format!("完成安全备份写入失败: {}", e))?;
    tracing::info!("Pre-restore backup created: {}", backup_name);
    Ok(())
}

// 删除备份文件：验证文件名安全性后删除
pub async fn delete_backup(
    _admin: AdminUser,
    Path(filename): Path<String>,
) -> impl IntoResponse {
    // 允许删除常规备份和恢复前的安全备份
    if !is_valid_backup_name(&filename) && !filename.starts_with(PRE_RESTORE_PREFIX) {
        return Html(render_error("无效的备份文件名")).into_response();
    }
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Html(render_error("无效的备份文件名")).into_response();
    }

    let path = format!("{}/{}", backups_dir(), filename);
    let _ = tokio::task::spawn_blocking(move || fs::remove_file(&path)).await;

    Redirect::to("/admin/backup").into_response()
}
