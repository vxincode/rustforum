# 贡献指南

感谢你对 RustForum 项目的关注！欢迎提交 Issue 和 Pull Request。

## 开发环境搭建

```bash
# 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 克隆项目
git clone https://github.com/vxincode/rustforum.git
cd rustforum

# 编译运行
cargo run
```

首次运行会自动创建数据库并跳转到安装向导，按向导完成配置即可。

## 代码结构

| 目录/文件 | 说明 |
|-----------|------|
| `src/main.rs` | 程序入口，路由注册 |
| `src/handlers/` | 请求处理器（按功能分模块） |
| `src/middleware/` | 中间件（认证、CSRF、限流、安装检测） |
| `src/models/` | 数据模型 |
| `src/templates.rs` | HTML 模板渲染（约 4000 行） |
| `src/db.rs` | 数据库初始化、迁移 |
| `src/config.rs` | 配置管理 |
| `src/site_config.rs` | 站点设置全局缓存 |
| `migrations/` | SQL 迁移文件 |

## 添加新功能的流程

1. **新增数据表**：在 `migrations/` 下创建 SQL 文件，用 `CREATE TABLE IF NOT EXISTS`，并在 `src/db.rs` 的 `ddl_migrations` 数组中注册
2. **数据模型**：在 `src/models/` 下创建或修改 `.rs` 文件
3. **请求处理器**：在 `src/handlers/` 下创建或修改 `.rs` 文件，并在 `mod.rs` 中注册模块
4. **注册路由**：在 `src/main.rs` 中添加 `.route()` 调用
5. **页面模板**：在 `src/templates.rs` 中添加 `render_*` 函数

## 数据库变更

- **新表**：创建 `migrations/0xx_xxx.sql`，使用 `CREATE TABLE IF NOT EXISTS`
- **加列**：在 `src/db.rs` 的 `alter_migrations` 数组中添加条目，会自动检测并执行
- 所有迁移都是幂等的，可安全重复执行

## 代码风格

- Handler 返回 `impl IntoResponse`
- 数据库查询使用 `sqlx::query_as` 参数化绑定，防止 SQL 注入
- HTML 输出统一使用 `html_escape()` 转义
- 错误处理使用 `.ok()` 或 `.unwrap_or_default()` 降级，避免 panic
- 日志使用 `tracing::info/warn/error`
- Rust 代码格式化使用 `cargo fmt`

## 提交 PR 的流程

1. Fork 本仓库
2. 创建功能分支：`git checkout -b feature/your-feature`
3. 提交改动：`git commit -m "Add some feature"`
4. 推送分支：`git push origin feature/your-feature`
5. 在 GitHub 上发起 Pull Request

### PR 要求

- 一个 PR 只做一件事，保持改动范围最小化
- 提交信息清晰描述改动内容
- 确保 `cargo build` 无错误无警告
- 如果涉及数据库变更，需提供对应的迁移文件

## 报告 Bug

请通过 [GitHub Issues](https://github.com/vxincode/rustforum/issues) 提交，包含以下信息：

- 问题描述
- 复现步骤
- 期望行为
- 实际行为
- 运行环境（操作系统、Rust 版本等）

## 许可证

提交代码即表示你同意按照 [MIT 许可证](LICENSE) 授权你的贡献。
