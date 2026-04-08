# RustForum

一个基于 **Rust + Axum + SQLite** 的现代化高性能论坛系统。服务端渲染、单二进制部署、零外部服务依赖。

[English](README.md) | 中文

## 功能特性

- **用户系统** — 注册、登录、个人资料、头像上传、每日签到、积分体系
- **论坛核心** — 多版块分类、发帖回复、Markdown 渲染、置顶/精华/关闭
- **互动功能** — 私信、通知（回复/引用/私信）、举报
- **管理后台** — 仪表盘、版块/用户/帖子管理、登录日志、邀请码、黑名单、AI 审核、数据备份
- **AI 共享** — 用积分兑换 Prompt/Skill 分享
- **移动端适配** — 响应式布局，适配各种屏幕尺寸

## 快速开始

```bash
# 1. 安装 Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2. 克隆并编译
git clone https://github.com/vxincode/rustforum.git
cd rustforum
cargo build --release

# 3. 运行
./target/release/rustforum
```

打开 `http://localhost:3000`，按照安装向导创建管理员账号并配置站点。

## 技术栈

| 组件 | 技术 |
|------|------|
| Web 框架 | Axum 0.8 |
| 数据库 | SQLite（通过 SQLx 0.8） |
| Markdown | pulldown-cmark 0.13 |
| 认证 | bcrypt + Cookie 会话 |
| 前端 | Tailwind CSS + 原生 JavaScript |
| 缓存 | Redis（可选，优雅降级） |

## 配置

所有配置通过环境变量读取（也可使用 `.env` 文件）：

| 变量名 | 默认值 | 说明 |
|--------|--------|------|
| `LISTEN_ADDR` | `0.0.0.0:3000` | 监听地址 |
| `DATABASE_URL` | `sqlite:forum.db?mode=rwc` | SQLite 数据库路径 |
| `SESSION_SECRET` | `rustforum-secret-change-me` | 会话密钥（**生产环境务必修改**） |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis 地址（可选） |

## 项目结构

```
src/
  main.rs           # 入口：数据库初始化、路由注册、HTTP 服务
  config.rs         # 应用配置与状态
  db.rs             # 数据库连接池、迁移、种子数据
  site_config.rs    # 全局站点设置缓存
  templates.rs      # 所有 HTML 渲染（约 4000 行）
  cache.rs          # Redis 缓存辅助
  handlers/         # 请求处理器（按功能分模块）
  middleware/        # 认证、CSRF、限流、安装检测中间件
  models/           # 数据模型
migrations/         # SQL 迁移文件
static/             # CSS、JS、字体、图片、头像
```

## 部署

详见 [部署指南](DEPLOY_CN.md)。

## 贡献

欢迎贡献！请阅读 [贡献指南](CONTRIBUTING_CN.md)。

## 许可证

[MIT](LICENSE)
