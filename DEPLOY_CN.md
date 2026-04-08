# 部署指南（宝塔面板）

## 环境要求

- 宝塔面板 7.x/8.x（Linux）
- CentOS 7+ / Ubuntu 18+ / Debian 10+
- 内存 >= 1GB（编译需要）
- Rust 工具链

---

## 一、安装 Rust

SSH 登录服务器，执行：

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

选择默认安装（输入 1），安装完成后加载环境：

```bash
source $HOME/.cargo/env
```

验证安装：

```bash
rustc --version
cargo --version
```

---

## 二、上传项目

将整个项目上传到服务器，例如 `/www/rustforum/`。

可以通过宝塔面板「文件」功能上传 ZIP 包，或用 Git：

```bash
cd /www
git clone https://github.com/vxincode/rustforum.git
```

确保目录结构如下：

```
/www/rustforum/
├── Cargo.toml
├── src/
├── static/
│   ├── avatars/
│   ├── css/
│   ├── fonts/
│   ├── images/
│   └── js/
├── migrations/
└── .env          ← 稍后创建
```

---

## 三、编译项目

```bash
cd /www/rustforum

# 编译 release 版本（优化性能）
cargo build --release
```

> 首次编译需要下载依赖，耗时 5-15 分钟（取决于服务器配置）。

编译完成后二进制文件位于：

```
target/release/rustforum
```

---

## 四、创建环境配置

```bash
cd /www/rustforum
cat > .env << 'EOF'
# 监听地址（仅本地，由 Nginx 反代）
LISTEN_ADDR=127.0.0.1:3000

# 数据库路径（绝对路径）
DATABASE_URL=sqlite:/www/rustforum/data/forum.db?mode=rwc

# 头像目录（绝对路径）
AVATAR_DIR=/www/rustforum/static/avatars

# 头像大小限制（字节）
MAX_AVATAR_SIZE=524288

# Session 密钥（请修改为随机字符串）
SESSION_SECRET=改成你自己的随机字符串

# 站点信息
SITE_NAME=我的论坛
SITE_DESC=一个不错的论坛
EOF
```

> **重要**：`SESSION_SECRET` 必须修改为你自己的随机字符串，否则有安全风险。

创建数据目录：

```bash
mkdir -p /www/rustforum/data
mkdir -p /www/rustforum/backups
```

---

## 五、设置目录权限

```bash
# 将 www 用户设为目录所有者（宝塔默认用户）
chown -R www:www /www/rustforum

# 确保可写
chmod -R 755 /www/rustforum
chmod -R 777 /www/rustforum/static/avatars
chmod -R 777 /www/rustforum/data
chmod -R 777 /www/rustforum/backups
```

---

## 六、创建 systemd 服务

```bash
cat > /etc/systemd/system/rustforum.service << 'EOF'
[Unit]
Description=RustForum
After=network.target

[Service]
Type=simple
User=www
Group=www
WorkingDirectory=/www/rustforum
EnvironmentFile=/www/rustforum/.env
ExecStart=/www/rustforum/target/release/rustforum
Restart=always
RestartSec=5
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
EOF
```

启动服务：

```bash
systemctl daemon-reload
systemctl enable rustforum
systemctl start rustforum
```

检查运行状态：

```bash
systemctl status rustforum
```

看到 `active (running)` 表示启动成功。

查看日志：

```bash
journalctl -u rustforum -f
```

---

## 七、宝塔配置 Nginx 反向代理

### 7.1 在宝塔面板添加站点

1. 打开宝塔面板 →「网站」→「添加站点」
2. 填入你的域名（如 `forum.example.com`）
3. PHP 版本选「纯静态」
4. 数据库不创建
5. 点击「提交」

### 7.2 配置反向代理

1. 点击站点名称 →「反向代理」→「添加反向代理」
2. 配置：
   - 代理名称：`rustforum`
   - 目标 URL：`http://127.0.0.1:3000`
   - 发送域名：`$host`
3. 点击「保存」

### 7.3 修改 Nginx 配置（重要）

点击站点 →「配置文件」，在 `server { }` 块内替换为：

```nginx
    # 反向代理到 Rust 后端
    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # 上传文件大小限制（备份恢复）
        client_max_body_size 100m;

        # 超时设置
        proxy_connect_timeout 60s;
        proxy_read_timeout 120s;
        proxy_send_timeout 120s;
    }

    # 静态文件由 Nginx 直接提供（可选优化）
    location /static/ {
        alias /www/rustforum/static/;
        expires 7d;
        add_header Cache-Control "public, immutable";
    }
```

> **注意**：如果配置了 Nginx 直接提供静态文件，需要确保 Nginx 对 `/static/` 路径不再走反向代理。

### 7.4 配置 SSL（HTTPS）

1. 站点 →「SSL」→「Let's Encrypt」
2. 勾选域名 → 申请
3. 开启「强制 HTTPS」

---

## 八、验证部署

1. 浏览器访问 `https://你的域名/`
2. 将自动跳转到安装向导页面
3. 按照向导创建管理员账号并配置站点信息
4. 点击「进入论坛」开始使用

---

## 九、日常维护

### 更新代码后重新部署

```bash
cd /www/rustforum
git pull                          # 拉取最新代码
cargo build --release             # 编译
systemctl restart rustforum       # 重启服务
```

### 查看日志

```bash
# 实时日志
journalctl -u rustforum -f

# 最近 100 行
journalctl -u rustforum -n 100
```

### 备份与恢复

在管理后台 →「数据备份」页面操作：
- 创建备份：生成 ZIP 文件
- 下载备份：保存到本地
- 恢复数据：上传 ZIP 恢复

手动备份数据库：

```bash
cp /www/rustforum/data/forum.db /www/backup/forum_$(date +%Y%m%d).db
```

### 数据库维护

```bash
# 进入数据库命令行
sqlite3 /www/rustforum/data/forum.db

# 清理过期 session
DELETE FROM sessions WHERE expires_at < datetime('now');

# 退出
.quit
```

---

## 十、常见问题

### Q: 编译报错 `linker 'cc' not found`

安装编译工具：

```bash
# CentOS
yum groupinstall -y "Development Tools"

# Ubuntu/Debian
apt install -y build-essential
```

### Q: 编译时内存不足

使用 swap：

```bash
fallocate -l 2G /swapfile
chmod 600 /swapfile
mkswap /swapfile
swapon /swapfile
```

或者本地交叉编译后上传二进制文件。

### Q: 本地交叉编译上传（推荐小内存服务器）

在本地 Windows/Mac 上编译 Linux 二进制：

```bash
# 添加 Linux 目标
rustup target add x86_64-unknown-linux-gnu

# 编译
cargo build --release --target x86_64-unknown-linux-gnu
```

然后上传 `target/x86_64-unknown-linux-gnu/release/rustforum` 到服务器。

> 如果交叉编译遇到问题，可以用 GitHub Actions CI 自动编译，或在一台临时 2G 内存服务器上编译。

### Q: 启动后 502 Bad Gateway

检查服务是否在运行：

```bash
systemctl status rustforum
journalctl -u rustforum -n 20
```

常见原因：
- `.env` 文件路径不对
- 数据库目录不存在
- 端口被占用（`ss -tlnp | grep 3000`）

### Q: 上传头像失败

检查目录权限：

```bash
chmod -R 777 /www/rustforum/static/avatars
chown -R www:www /www/rustforum/static/avatars
```

### Q: 恢复备份后数据没变化

恢复数据库后需要重启服务：

```bash
systemctl restart rustforum
```

---

## 目录结构总览

```
/www/rustforum/
├── .env                          ← 环境配置
├── target/release/rustforum      ← 可执行文件
├── data/
│   └── forum.db                  ← SQLite 数据库
├── static/
│   ├── avatars/                  ← 用户头像（可写）
│   ├── css/
│   ├── fonts/
│   ├── images/
│   └── js/
├── backups/                      ← 系统备份文件（可写）
├── migrations/                   ← 数据库迁移脚本
└── src/                          ← 源代码
```
