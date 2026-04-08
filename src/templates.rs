use crate::handlers::admin::{SiteStats, AdminThreadRow, LoginLogRow};
use crate::models::ai_share::{AiShare, AiShareList};
use crate::models::forum::Forum;
use crate::models::message::Message;
use crate::models::post::Post;
use crate::models::report::ReportWithReporter;
use crate::models::blacklist::{BlacklistEntry, MutedUserWithInfo};
use crate::models::thread::{Thread, ThreadList};
use crate::models::user::User;

// =====================================================================
// Avatar helper
// =====================================================================

fn avatar_html(avatar: &str, _user_id: i64, username: &str, size_css: &str) -> String {
    let initial = username.chars().next().unwrap_or('U').to_uppercase();
    if !avatar.is_empty() {
        format!(
            r#"<img src="/static/avatars/{avatar}" class="rounded-full object-cover {size_css}" alt="{username}" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"><span class="rounded-full bg-black text-white flex items-center justify-center font-bold {size_css}" style="display:none">{initial}</span>"#,
            avatar = avatar,
            size_css = size_css,
            username = html_escape(username),
            initial = initial,
        )
    } else {
        format!(
            r#"<span class="rounded-full overflow-hidden flex items-center justify-center {size_css}" data-multiavatar="{username}"></span>"#,
            size_css = size_css,
            username = html_escape(username),
        )
    }
}

// =====================================================================
// Layout - sticky header, Tailwind, FA icons
// =====================================================================

fn layout(title: &str, user: Option<&User>, content: &str, nav_section: &str) -> String {
    let site_name = crate::site_config::site_name();
    let site_description = crate::site_config::site_description();
    let site_keywords = crate::site_config::site_keywords();
    let site_footer = crate::site_config::site_footer();

    // CSRF meta tag - derive token from session cookie or use anonymous key
    let csrf_meta = match user {
        Some(u) => format!(r#"<meta name="csrf-token" content="{}">"#,
            crate::middleware::csrf::generate_token(&u.id.to_string())),
        None => format!(r#"<meta name="csrf-token" content="{}">"#,
            crate::middleware::csrf::generate_token("anonymous")),
    };

    let home_cls = if nav_section == "home" { "font-medium border-b-2 border-black pb-1" } else { "text-gray-500 hover:text-black transition-colors pb-1" };
    let forums_cls = if nav_section == "forums" { "font-medium border-b-2 border-black pb-1" } else { "text-gray-500 hover:text-black transition-colors pb-1" };
    let share_cls = if nav_section == "ai" { "font-medium border-b-2 border-black pb-1" } else { "text-gray-500 hover:text-black transition-colors pb-1" };

    let nav_links = format!(r#"
      <a href="/" class="{home_cls}">首页</a>
      <a href="/forums" class="{forums_cls}">版块</a>
      <a href="/ai" class="{share_cls}">AI 共享</a>"#);

    let right_nav = match user {
        Some(u) => format!(
            r#"<div class="flex items-center gap-5">
        <div class="relative hidden sm:block">
          <input type="text" id="searchInput" placeholder="搜索话题、帖子或用户..." class="bg-gray-100 rounded-full px-4 py-2 text-sm outline-none w-40 md:w-56">
          <i class="fa fa-search absolute right-3 top-2.5 text-gray-400 cursor-pointer" onclick="doSearch()"></i>
        </div>
        <!-- Notification bell -->
        <div class="relative" id="notifBellWrap">
          <button type="button" onclick="toggleNotifPanel()" class="relative text-gray-500 hover:text-black transition-colors">
            <i class="fa fa-bell text-lg"></i>
            <span id="notifBadge" class="hidden absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full w-4 h-4 flex items-center justify-center" style="font-size:10px"></span>
          </button>
          <!-- Notification dropdown -->
          <div id="notifPanel" class="hidden absolute right-0 top-full mt-2 w-80 bg-white border border-gray-200 rounded-xl shadow-xl z-50 overflow-hidden">
            <div class="flex items-center justify-between px-4 py-3 border-b border-gray-100">
              <span class="font-medium text-sm">通知</span>
              <button type="button" onclick="markAllRead()" class="text-xs text-gray-400 hover:text-black">全部已读</button>
            </div>
            <div id="notifList" class="max-h-72 overflow-y-auto">
              <div class="px-4 py-6 text-center text-gray-400 text-xs">加载中...</div>
            </div>
          </div>
        </div>
        <a href="/profile" class="hidden sm:flex items-center gap-2 text-sm font-medium hover:text-gray-600 transition-colors">
          {nav_avatar}
        </a>
        <a href="/auth/logout" class="text-sm text-gray-500 hover:text-black"><i class="fa fa-sign-out"></i></a>
        {admin_link}
      </div>"#,
            nav_avatar = avatar_html(&u.avatar, u.id, &u.username, "w-8 h-8 text-xs"),
            admin_link = if u.is_admin() { r#"<a href="/admin" class="text-sm text-gray-500 hover:text-black"><i class="fa fa-cog"></i></a>"# } else { "" },
        ),
        None => r#"<div class="flex items-center gap-4">
        <div class="relative hidden sm:block">
          <input type="text" id="searchInput" placeholder="搜索话题、帖子或用户..." class="bg-gray-100 rounded-full px-4 py-2 text-sm outline-none w-40 md:w-56">
          <i class="fa fa-search absolute right-3 top-2.5 text-gray-400 cursor-pointer" onclick="doSearch()"></i>
        </div>
        <a href="/auth/login" class="text-sm text-gray-500 hover:text-black transition-colors">登录</a>
        <a href="/auth/register" class="bg-black text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">注册</a>
      </div>"#.to_string(),
    };

    format!(r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title} | {site_name}</title>
  <meta name="description" content="{site_description}">
  <meta name="keywords" content="{site_keywords}">
  {csrf_meta}
  <script src="/static/css/tailwind.js"></script>
  <link href="/static/css/font-awesome.min.css" rel="stylesheet">
  <script>
    tailwind.config = {{
      theme: {{
        extend: {{
          colors: {{
            primary: '#000000',
            secondary: '#666666',
            muted: '#f5f5f5',
          }},
          fontFamily: {{
            sans: ['Inter', 'system-ui', 'sans-serif'],
          }},
        }},
      }}
    }}
  </script>
  <style type="text/tailwindcss">
    @layer utilities {{
      .item-hover {{ transition: all 0.2s ease; }}
      .item-hover:hover {{ background: #f9f9f9; }}
      .fade-in {{ animation: fadeIn 0.3s ease; }}
      @keyframes fadeIn {{ from {{ opacity:0; transform:translateY(8px) }} to {{ opacity:1; transform:translateY(0) }} }}
    }}
  </style>
  <style>
    #userCardPopup {{
      position: fixed;
      z-index: 1000;
      pointer-events: none;
      opacity: 0;
      transition: opacity 0.15s ease;
      min-width: 220px;
    }}
    #userCardPopup.show {{
      opacity: 1;
    }}
    .markdown-body h1 {{ font-size:1.5em; font-weight:700; margin:0.8em 0 0.4em; }}
    .markdown-body h2 {{ font-size:1.3em; font-weight:700; margin:0.7em 0 0.3em; }}
    .markdown-body h3 {{ font-size:1.15em; font-weight:600; margin:0.6em 0 0.3em; }}
    .markdown-body h4 {{ font-size:1em; font-weight:600; margin:0.5em 0 0.2em; }}
    .markdown-body p {{ margin:0.5em 0; }}
    .markdown-body ul {{ list-style:disc; padding-left:1.5em; margin:0.5em 0; }}
    .markdown-body ol {{ list-style:decimal; padding-left:1.5em; margin:0.5em 0; }}
    .markdown-body li {{ margin:0.2em 0; }}
    .markdown-body code {{ background:#f3f4f6; padding:0.15em 0.4em; border-radius:3px; font-size:0.9em; font-family:'Fira Code',Consolas,monospace; }}
    .markdown-body pre {{ background:#1e1e1e; color:#d4d4d4; padding:1em; border-radius:8px; overflow-x:auto; margin:0.8em 0; }}
    .markdown-body pre code {{ background:none; padding:0; color:inherit; font-size:0.875em; }}
    .markdown-body blockquote {{ border-left:3px solid #d1d5db; padding:0.5em 1em; margin:0.8em 0; color:#6b7280; background:#f9fafb; border-radius:0 6px 6px 0; }}
    .markdown-body a {{ color:#2563eb; text-decoration:underline; }}
    .markdown-body a:hover {{ color:#1d4ed8; }}
    .markdown-body hr {{ border:none; border-top:1px solid #e5e7eb; margin:1em 0; }}
    .markdown-body table {{ border-collapse:collapse; margin:0.8em 0; width:100%; }}
    .markdown-body th, .markdown-body td {{ border:1px solid #e5e7eb; padding:0.5em 0.75em; text-align:left; }}
    .markdown-body th {{ background:#f9fafb; font-weight:600; }}
    .markdown-body img {{ max-width:100%; border-radius:6px; }}
    .markdown-body input[type="checkbox"] {{ margin-right:0.4em; }}
    .markdown-body del {{ color:#9ca3af; }}
    .toolbar-btn {{
      width:32px; height:32px; display:flex; align-items:center; justify-content:center;
      border-radius:6px; color:#6b7280; transition:all 0.15s;
      border:none; background:transparent; cursor:pointer; font-size:13px;
    }}
    .toolbar-btn:hover {{ background:#f3f4f6; color:#111827; }}
    .notif-item {{ display:flex; align-items:flex-start; gap:10px; padding:10px 16px; border-bottom:1px solid #f3f4f6; text-decoration:none; color:#111; cursor:pointer; transition:background 0.15s; }}
    .notif-item:hover {{ background:#f9fafb; }}
    .notif-item.unread {{ background:#fffbeb; }}
    .notif-item.unread:hover {{ background:#fef3c7; }}
    .notif-dot {{ width:8px; height:8px; border-radius:50%; background:#ef4444; flex-shrink:0; margin-top:6px; }}
    [data-multiavatar] svg {{ width:100%; height:100%; display:block; }}
  </style>
</head>
<body class="text-black min-h-screen flex flex-col" style="background-color:#fafafa;background-image:linear-gradient(to right, #e7e5e4 1px, transparent 1px),linear-gradient(to bottom, #e7e5e4 1px, transparent 1px);background-size:40px 40px;">
<!-- 导航栏 -->
<header class="sticky top-0 z-50 bg-white border-b border-gray-200 backdrop-blur-sm bg-opacity-90">
  <div class="container mx-auto px-4 py-4 flex items-center justify-between">
    <a href="/" class="flex items-center gap-2 font-semibold text-lg">
      <i class="fa fa-comments"></i>
      {site_name}
    </a>
    <nav class="hidden md:flex items-center gap-8 text-sm">
      {nav_links}
    </nav>
    <div class="flex items-center">
      <button type="button" class="md:hidden text-gray-500 hover:text-black mr-3 p-1" onclick="document.getElementById('mobileMenu').classList.toggle('hidden')">
        <i class="fa fa-bars text-xl"></i>
      </button>
      {right_nav}
    </div>
  </div>
  <!-- 手机端导航菜单 -->
  <div id="mobileMenu" class="hidden md:hidden border-t border-gray-100 px-4 py-3 space-y-2 text-sm">
    <a href="/" class="block py-2 hover:text-black">首页</a>
    <a href="/forums" class="block py-2 hover:text-black">版块</a>
    <a href="/ai" class="block py-2 hover:text-black">AI 共享</a>
  </div>
</header>

<!-- 通知 -->
<div id="toast" class="fixed top-20 right-4 z-[999] hidden">
  <div class="bg-black text-white px-5 py-3 rounded-lg shadow-lg text-sm" id="toastMsg"></div>
</div>

<main class="flex-1">
{content}
</main>

<!-- 页脚 -->
<footer class="border-t border-gray-200 mt-12 py-8 text-center text-sm text-gray-500 bg-white">
  <div class="container mx-auto px-4">
    <div class="flex flex-col md:flex-row justify-center items-center gap-4 md:gap-8 mb-4">
      <a href="/about" class="hover:text-black">关于我们</a>
      <a href="/terms" class="hover:text-black">使用条款</a>
      <a href="/privacy" class="hover:text-black">隐私政策</a>
      <a href="/contact" class="hover:text-black">联系我们</a>
    </div>
    <p>&copy; {year} {site_name} 版权所有 | {site_footer}</p>
  </div>
</footer>

<!-- User hover card -->
<div id="userCardPopup" class="bg-white border border-gray-200 rounded-xl shadow-lg p-4"></div>
<script src="/static/js/multiavatar.min.js"></script>
<script src="/static/js/app.js"></script>
<script>
// CSRF Protection: auto-inject token into all forms
(function() {{
  var meta = document.querySelector('meta[name="csrf-token"]');
  if (meta) {{
    var token = meta.getAttribute('content');
    document.addEventListener('submit', function(e) {{
      var form = e.target;
      while (form && form.tagName !== 'FORM') form = form.parentElement;
      if (form && form.tagName === 'FORM') {{
        var input = form.querySelector('input[name="csrf_token"]');
        if (!input) {{
          input = document.createElement('input');
          input.type = 'hidden';
          input.name = 'csrf_token';
          form.appendChild(input);
        }}
        input.value = token;
      }}
    }}, true);
  }}
}})();
</script>
</body>
</html>"##,
        title = html_escape(title),
        site_description = html_escape(&site_description),
        site_keywords = html_escape(&site_keywords),
        nav_links = nav_links,
        right_nav = right_nav,
        content = content,
        year = chrono::Local::now().format("%Y"),
    )
}

// =====================================================================
// Unified sidebar — same on every content page
// =====================================================================

fn unified_sidebar(user: Option<&User>, page_extra: &str) -> String {
    let user_card = match user {
        Some(u) => format!(r#"<div class="bg-white rounded-xl p-5 mb-5 border border-gray-100">
      <div class="flex items-center gap-3">
        {sidebar_avatar}
        <div>
          <h3 class="font-medium text-sm">{username}</h3>
          <div class="flex items-center gap-1 flex-wrap mt-0.5">
            {epithet_badge}
            <span class="text-xs text-gray-500">{group_name} · {rank_title} · {credits} 积分</span>
          </div>
        </div>
      </div>
      <div class="w-full h-px bg-gray-200 my-3"></div>
      <div class="grid grid-cols-2 gap-2">
        <a href="/profile" class="bg-black text-white py-1.5 rounded-lg text-xs font-medium hover:bg-gray-800 transition-colors text-center block">个人中心</a>
        <a href="/thread/mine" class="bg-gray-100 text-black py-1.5 rounded-lg text-xs font-medium hover:bg-gray-200 transition-colors text-center block">我的帖子</a>
      </div>
      <div class="grid grid-cols-2 gap-2 mt-2">
        <button id="checkinBtn" onclick="doCheckin()" class="bg-gray-100 text-black py-1.5 rounded-lg text-xs font-medium hover:bg-gray-200 transition-colors flex items-center justify-center gap-1">
          <i class="fa fa-calendar-check-o"></i> <span id="checkinBtnText">签到</span>
        </button>
        <a href="/messages" class="bg-gray-100 text-black py-1.5 rounded-lg text-xs font-medium hover:bg-gray-200 transition-colors flex items-center justify-center gap-1 relative">
          <i class="fa fa-bell"></i> 消息<span id="msgBadge" class="hidden absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full w-4 h-4 flex items-center justify-center"></span>
        </a>
      </div>
      <div id="checkinInfo" class="text-xs text-gray-400 mt-2 text-center hidden"></div>
    </div>"#,
            sidebar_avatar = avatar_html(&u.avatar, u.id, &u.username, "w-12 h-12 text-lg"),
            username = html_escape(&u.username),
            group_name = u.group_name(),
            rank_title = u.display_title(),
            credits = u.credits,
            epithet_badge = u.epithet_badge(),
        ),
        None => r#"<div class="bg-white rounded-xl p-5 mb-5 border border-gray-100">
      <div class="flex items-center gap-3">
        <span class="w-12 h-12 rounded-full bg-gray-200 text-gray-400 flex items-center justify-center text-lg"><i class="fa fa-user"></i></span>
        <div>
          <h3 class="font-medium text-sm">未登录</h3>
          <p class="text-xs text-gray-500 mt-0.5">登录后参与社区讨论</p>
        </div>
      </div>
      <div class="w-full h-px bg-gray-200 my-3"></div>
      <div class="grid grid-cols-2 gap-2">
        <a href="/auth/login" class="bg-black text-white py-1.5 rounded-lg text-xs font-medium hover:bg-gray-800 transition-colors text-center block">登录</a>
        <a href="/auth/register" class="bg-gray-100 text-black py-1.5 rounded-lg text-xs font-medium hover:bg-gray-200 transition-colors text-center block">注册</a>
      </div>
    </div>"#.to_string(),
    };

    format!(r#"
  <aside class="lg:col-span-3 hidden lg:block">
    {user_card}
    {page_extra}
    <!-- 分类导航 -->
    <div class="bg-white rounded-xl p-5 mb-5 border border-gray-100">
      <h3 class="font-medium text-sm mb-2">论坛分类</h3>
      <div class="w-full h-px bg-gray-200 my-2"></div>
      <div class="space-y-1 text-xs" id="categoryList">
        <div class="p-2 text-gray-400">加载中...</div>
      </div>
    </div>

    <!-- 新注册用户 -->
    <div class="bg-white rounded-xl p-5 mb-5 border border-gray-100">
      <h3 class="font-medium text-sm mb-2">新会员</h3>
      <div class="w-full h-px bg-gray-200 my-2"></div>
      <div class="space-y-2 text-xs" id="newUsers">
        <div class="p-2 text-gray-400">加载中...</div>
      </div>
    </div>

    <!-- 积分排行 TOP 10 -->
    <div class="bg-white rounded-xl p-5 mb-5 border border-gray-100">
      <h3 class="font-medium text-sm mb-2">积分排行</h3>
      <div class="w-full h-px bg-gray-200 my-2"></div>
      <div class="space-y-1.5 text-xs" id="leaderboard">
        <div class="p-2 text-gray-400">加载中...</div>
      </div>
    </div>

    <!-- 社区统计 -->
    <div class="bg-white rounded-xl p-5 mb-5 border border-gray-100">
      <h3 class="font-medium text-sm mb-2">社区数据</h3>
      <div class="w-full h-px bg-gray-200 my-2"></div>
      <div class="grid grid-cols-2 gap-2 text-center text-xs" id="siteStats">
        <div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">-</p><p class="text-gray-500 text-xs">帖子</p></div>
        <div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">-</p><p class="text-gray-500 text-xs">回复</p></div>
        <div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">-</p><p class="text-gray-500 text-xs">会员</p></div>
        <div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">-</p><p class="text-gray-500 text-xs">今日签到</p></div>
      </div>
    </div>

    <!-- 友情链接 -->
    <div class="bg-white rounded-xl p-5 border border-gray-100">
      <h3 class="font-medium text-sm mb-2">友情链接</h3>
      <div class="w-full h-px bg-gray-200 my-2"></div>
      <div class="space-y-1 text-xs" id="friendlyLinks">
        <div class="p-2 text-gray-400">加载中...</div>
      </div>
    </div>
  </aside>"#,
        user_card = user_card,
        page_extra = page_extra,
    )
}

// =====================================================================
// Page wrapper: main content area (col-span-9) + unified sidebar
// =====================================================================

fn page_with_sidebar(title: &str, main_content: &str, user: Option<&User>, sidebar_extra: &str, nav_section: &str) -> String {
    let content = format!(
        r#"<div class="container mx-auto px-4 py-10 grid grid-cols-1 lg:grid-cols-12 gap-6">
  <div class="lg:col-span-9">{main}</div>
  {sidebar}
</div>"#,
        main = main_content,
        sidebar = unified_sidebar(user, sidebar_extra),
    );
    layout(title, user, &content, nav_section)
}

// =====================================================================
// Index Page
// =====================================================================

pub fn render_index(recent: &[ThreadList], _hot: &[ThreadList], user: Option<&User>, page: i64, total_pages: i64) -> String {
    let thread_rows = if recent.is_empty() {
        r#"<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无帖子，快来发布第一个吧</div>"#.to_string()
    } else {
        recent.iter().map(|t| thread_row_html(t)).collect::<Vec<_>>().join("\n")
    };

    let new_thread_btn = match user {
        Some(_) => r#"<button onclick="location.href='/new'" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors"><i class="fa fa-plus mr-1"></i>发布新帖</button>"#,
        None => r#"<a href="/auth/login" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors inline-block"><i class="fa fa-plus mr-1"></i>发布新帖</a>"#,
    };

    let pagination = pagination_html(page, total_pages, "/");

    let main = format!(r#"
    <div class="flex justify-between items-center mb-6">
      <div class="flex gap-4">
        <button id="tabLatest" onclick="switchTab('latest')" class="text-base font-medium border-b-2 border-black pb-1">最新发布</button>
        <button id="tabHot" onclick="switchTab('hot')" class="text-base text-gray-500 hover:text-black pb-1">热门帖子</button>
        <button id="tabEssence" onclick="switchTab('essence')" class="text-base text-gray-500 hover:text-black pb-1">精华帖子</button>
      </div>
      {new_thread_btn}
    </div>

    <div class="bg-white border border-gray-200 rounded-lg overflow-hidden fade-in" id="threadList">
      {thread_rows}
    </div>

    {pagination}"#,
        thread_rows = thread_rows,
        new_thread_btn = new_thread_btn,
        pagination = pagination,
    );

    page_with_sidebar("首页", &main, user, "", "home")
}

// =====================================================================
// Thread row helper
// =====================================================================

fn thread_row_html(t: &ThreadList) -> String {
    let username_raw = t.username.as_deref().unwrap_or("未知");
    let username = html_escape(username_raw);
    let badges = if t.is_top == 1 {
        r#"<span class="text-xs bg-red-100 text-red-600 px-1.5 py-0.5 rounded">置顶</span> "#
    } else {
        ""
    };
    let essence_badge = if t.is_essence == 1 {
        r#"<span class="text-xs bg-orange-100 text-orange-600 px-1.5 py-0.5 rounded">精华</span> "#
    } else {
        ""
    };
    let closed_badge = if t.is_closed == 1 {
        r#"<span class="text-xs bg-gray-200 text-gray-500 px-1.5 py-0.5 rounded">已关闭</span> "#
    } else {
        ""
    };
    let avatar_span = match &t.avatar {
        Some(a) if !a.is_empty() => format!(
            r#"<img src="/static/avatars/{avatar}" class="w-8 h-8 rounded-full object-cover flex-shrink-0" alt="{username}" onerror="this.style.display='none';this.nextElementSibling.style.display='flex'"><span class="w-8 h-8 rounded-full overflow-hidden flex-shrink-0" data-multiavatar="{username}" style="display:none"></span>"#,
            avatar = a,
            username = username,
        ),
        _ => format!(
            r#"<span class="w-8 h-8 rounded-full overflow-hidden flex-shrink-0" data-multiavatar="{}"></span>"#,
            html_escape(username_raw),
        ),
    };
    format!(
        r#"<div class="item-hover px-5 py-4 border-b border-gray-100 cursor-pointer" onclick="location.href='/thread/{id}'">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3 sm:gap-4">
            {avatar}
            <div class="min-w-0">
              <h3 class="font-medium text-sm sm:text-base truncate">{badges}{essence_badge}{closed_badge}{title}</h3>
              <div class="flex items-center gap-2 sm:gap-3 mt-1 text-xs text-gray-500">
                <span>{username}</span>
              </div>
            </div>
          </div>
          <div class="hidden sm:flex items-center gap-4 text-xs text-gray-500 flex-shrink-0">
            <span><i class="fa fa-eye"></i> {views}</span>
            <span><i class="fa fa-comment"></i> {replies}</span>
            <span class="text-gray-400">{time}</span>
          </div>
          <div class="sm:hidden flex items-center gap-2 text-xs text-gray-400 flex-shrink-0">
            <span><i class="fa fa-comment"></i> {replies}</span>
          </div>
        </div>
      </div>"#,
        id = t.id,
        avatar = avatar_span,
        badges = badges,
        closed_badge = closed_badge,
        title = html_escape(&t.title),
        username = username,
        views = t.view_count,
        replies = t.reply_count,
        time = t.created_at.chars().take(10).collect::<String>(),
    )
}

// =====================================================================
// Forum List
// =====================================================================

pub fn render_forum_list(forums: &[Forum], user: Option<&User>) -> String {
    let rows = if forums.is_empty() {
        r#"<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无版块</div>"#.to_string()
    } else {
        forums.iter().map(|f| {
            let last = match (&f.last_post_at, &f.last_post_user) {
                (Some(t), u) if !u.is_empty() => format!("{} by {}", t.chars().take(10).collect::<String>(), html_escape(u)),
                _ => "暂无".to_string(),
            };
            format!(
                r#"<div class="item-hover px-5 py-4 border-b border-gray-100 cursor-pointer" onclick="location.href='/forum/{id}'">
        <div class="flex items-center justify-between">
          <div>
            <h3 class="font-medium text-base">{name}</h3>
            <p class="text-xs text-gray-500 mt-1">{desc}</p>
          </div>
          <div class="flex items-center gap-6 text-xs text-gray-500 flex-shrink-0">
            <span>{threads} 主题</span>
            <span>{posts} 帖子</span>
            <span class="text-gray-400">{last}</span>
          </div>
        </div>
      </div>"#,
                id = f.id,
                name = html_escape(&f.name),
                desc = html_escape(&f.description),
                threads = f.thread_count,
                posts = f.post_count,
                last = last,
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let main = format!(r#"
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-semibold">版块列表</h2>
    </div>
    <div class="bg-white border border-gray-200 rounded-lg overflow-hidden fade-in">
      {rows}
    </div>"#,
        rows = rows,
    );

    page_with_sidebar("版块", &main, user, "", "forums")
}

// =====================================================================
// Forum View (thread list for a forum)
// =====================================================================

pub fn render_forum_view(
    forum: &Forum,
    sticky: &[ThreadList],
    threads: &[ThreadList],
    page: i64,
    total_pages: i64,
    user: Option<&User>,
    can_post: bool,
) -> String {
    let mut rows = String::new();
    for t in sticky {
        rows.push_str(&thread_row_html(t));
    }
    if threads.is_empty() && sticky.is_empty() {
        rows.push_str(r#"<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无帖子</div>"#);
    } else {
        for t in threads {
            rows.push_str(&thread_row_html(t));
        }
    }

    let new_btn = if can_post {
        format!(r#"<button onclick="location.href='/forum/{}/new'" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors"><i class="fa fa-plus mr-1"></i>发布新帖</button>"#, forum.id)
    } else if user.is_none() {
        r#"<a href="/auth/login" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors inline-block">登录后发帖</a>"#.to_string()
    } else {
        String::new()
    };

    let pagination = pagination_html(page, total_pages, &format!("/forum/{}", forum.id));

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <span class="text-black">{name}</span></div>
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-semibold">{name}</h2>
      {new_btn}
    </div>
    <p class="text-sm text-gray-500 mb-6">{desc}</p>
    <div class="bg-white border border-gray-200 rounded-lg overflow-hidden fade-in">
      {rows}
    </div>
    {pagination}"#,
        name = html_escape(&forum.name),
        desc = html_escape(&forum.description),
        rows = rows,
        new_btn = new_btn,
        pagination = pagination,
    );

    let sidebar_extra = "";

    page_with_sidebar(&forum.name, &main, user, &sidebar_extra, "forums")
}

// =====================================================================
// Thread View
// =====================================================================

pub fn render_thread_view(
    thread: &Thread,
    posts: &[Post],
    page: i64,
    total_pages: i64,
    user: Option<&User>,
) -> String {
    let posts_html = if posts.is_empty() {
        r#"<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无内容</div>"#.to_string()
    } else {
        posts.iter().map(|p| {
            let username = html_escape(p.username.as_deref().unwrap_or("未知"));
            let group_name = match p.group_id.unwrap_or(3) { 1 => "管理员", 2 => "版主", _ => "会员" };
            let title_badge = if !p.custom_title.as_deref().unwrap_or("").is_empty() {
                format!("<span class=\"text-xs bg-gradient-to-r from-purple-500 to-indigo-600 px-1.5 py-0.5 rounded font-medium\">{}</span> ", html_escape(p.custom_title.as_deref().unwrap_or("")))
            } else {
                String::new()
            };
            let epithet_badge = if !p.epithet.as_deref().unwrap_or("").is_empty() {
                let ec = p.epithet_color.as_deref().unwrap_or("");
                let ec_val = if ec.is_empty() { "#8B5CF6" } else { ec };
                format!("<span class=\"inline-flex items-center px-2 py-0.5 rounded-full text-xs font-bold shadow-sm\" style=\"background:linear-gradient(135deg,{},{});color:white;letter-spacing:0.05em\">{}</span>",
                    ec_val,
                    ec_val,
                    html_escape(p.epithet.as_deref().unwrap_or(""))
                )
            } else {
                String::new()
            };
            // report button for logged in users
            let report_btn = match user {
                Some(_) => format!(r#"<span class="text-gray-300">|</span>
                <a href="javascript:void(0)" onclick="submitReport('post',{})" class="text-xs text-gray-400 hover:text-red-500"><i class="fa fa-flag"></i> 举报</a>"#, p.id),
                _ => String::new(),
            };
            let raw_username = p.username.as_deref().unwrap_or("未知");
            // PM link for logged-in users (not to self)
            let pm_link = match user {
                Some(u) if u.id != p.user_id => format!(r#"<a href="/messages/compose?to={}" class="text-xs text-gray-400 hover:text-black" title="发私信"><i class="fa fa-envelope-o"></i></a>"#, urlencoding(raw_username)),
                _ => String::new(),
            };

            // Ban / mute badges
            let ban_badge = if p.user_status.unwrap_or(1) == 0 {
                r#"<span class="text-xs bg-red-100 text-red-600 px-1.5 py-0.5 rounded">封禁</span>"#.to_string()
            } else if p.user_muted.as_deref().map_or(false, |m| !m.is_empty() || m == "") {
                r#"<span class="text-xs bg-orange-100 text-orange-600 px-1.5 py-0.5 rounded">禁言</span>"#.to_string()
            } else {
                String::new()
            };

            let is_op = p.is_first == 1;
            let op_badge = if is_op { r#"<span class="text-xs bg-blue-100 text-blue-600 px-1.5 py-0.5 rounded">楼主</span>"# } else { "" };
            let post_avatar = avatar_html(p.avatar.as_deref().unwrap_or(""), p.user_id, p.username.as_deref().unwrap_or("U"), "w-12 h-12 text-base");

            let raw_content = &p.content;
            // Truncate quoted content for the quote button
            let quote_preview = truncate_chars(raw_content, 50).replace('\n', " ");
            let quote_snippet = if quote_preview.chars().count() > 50 { format!("{}...", truncate_chars(&quote_preview, 50)) } else { quote_preview.clone() };

            // Action buttons for post owner
            let owner_actions = match user {
                Some(u) if u.id == p.user_id => {
                    if is_op {
                        format!(r#"<a href="javascript:void(0)" onclick="editPost({post_id}, {is_first})" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-edit"></i> 编辑</a>"#, post_id = p.id, is_first = p.is_first)
                    } else {
                        format!(r#"<a href="javascript:void(0)" onclick="editPost({post_id}, {is_first})" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-edit"></i> 编辑</a>
                <span class="text-gray-300">|</span>
                <a href="javascript:void(0)" onclick="confirmDeletePost({post_id})" class="text-xs text-gray-400 hover:text-red-500"><i class="fa fa-trash-o"></i> 删除</a>"#, post_id = p.id, is_first = p.is_first)
                    }
                }
                _ => String::new(),
            };

            // Admin action buttons
            let admin_actions = match user {
                Some(u) if u.is_admin() => {
                    let mut actions = String::new();
                    if p.user_id != user.as_ref().map(|u2| u2.id).unwrap_or(0) {
                        actions.push_str(&format!(r#"<span class="text-gray-300">|</span>
                <a href="javascript:void(0)" onclick="adminDeletePost({post_id})" class="text-xs text-red-400 hover:text-red-600"><i class="fa fa-trash-o"></i> 管理删除</a>"#, post_id = p.id));
                    }
                    actions
                }
                _ => String::new(),
            };

            let signature_html = match &p.signature {
                Some(sig) if !sig.is_empty() => format!(r#"<div class="mt-3 pt-2 border-t border-gray-100 text-xs text-gray-400"><i class="fa fa-pencil mr-1"></i>{}</div>"#, html_escape(sig)),
                _ => String::new(),
            };

            let action_row = format!(r#"<div class="flex items-center gap-3 mt-3 pt-2 border-t border-gray-50">
              <a href="javascript:void(0)" onclick="quoteReply('{username}', '{content}')" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-quote-left"></i> 引用</a>
              <span class="text-gray-300">|</span>
              <a href="javascript:void(0)" onclick="quickReply('{username}', {floor})" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-reply"></i> 回复</a>
              {owner}{admin}{report}
            </div>"#,
                username = html_escape(raw_username).replace('\'', "\\'"),
                content = html_escape(&quote_snippet).replace('\'', "\\'").replace('\n', " "),
                floor = p.floor,
                owner = if owner_actions.is_empty() { String::new() } else { format!("<span class=\"text-gray-300\">|</span>\n{}", owner_actions) },
                admin = if admin_actions.is_empty() { String::new() } else { format!("<span class=\"text-gray-300\">|</span>\n{}", admin_actions) },
                report = report_btn,
            );

            format!(
                r#"<div class="px-5 py-5 border-b border-gray-100 fade-in" id="floor-{floor}">
        <div class="flex flex-col sm:flex-row sm:gap-4">
          <div class="hidden sm:flex flex-shrink-0 flex-col items-center w-20">
            <a href="/user/{user_id}" class="block" data-user-card="{user_id}">{post_avatar}</a>
            <div class="text-xs font-medium mt-2 text-center leading-tight"><a href="/user/{user_id}" class="hover:text-black">{username}</a></div>
            <div class="flex items-center gap-1 flex-wrap justify-center">
              {title_badge}{epithet_badge}
              <span class="text-xs text-gray-400">{group_name}</span>
              {pm_link}
            </div>
            {ban_badge}
          </div>
          <div class="flex-1 min-w-0">
            <div class="flex items-center gap-2 text-xs text-gray-400 mb-2">
              <span class="sm:hidden">{mobile_avatar} <a href="/user/{user_id}" class="hover:text-black font-medium">{username}</a></span>
              <span>#{floor} 楼</span>{op_badge}
              <span class="ml-auto">{time}</span>
            </div>
            <div class="text-sm leading-relaxed break-words markdown-body">{content}</div>
            {signature_html}
            {action_row}
          </div>
        </div>
      </div>"#,
                floor = p.floor,
                user_id = p.user_id,
                post_avatar = post_avatar,
                mobile_avatar = avatar_html(p.avatar.as_deref().unwrap_or(""), p.user_id, p.username.as_deref().unwrap_or("U"), "w-6 h-6 text-xs inline-block"),
                username = username,
                group_name = group_name,
                op_badge = op_badge,
                time = p.created_at.chars().take(16).collect::<String>(),
                content = render_content(&p.content),
                signature_html = signature_html,
                action_row = action_row,
                title_badge = title_badge,
                epithet_badge = epithet_badge,
                pm_link = pm_link,
                ban_badge = ban_badge,
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let reply_form = match user {
        Some(_) if thread.is_closed == 0 => format!(r#"
    <div class="bg-white border border-gray-200 rounded-lg p-5 mt-6 fade-in" id="replySection">
      <h3 class="font-medium mb-4">回复帖子</h3>
      <form id="replyForm" accept-charset="UTF-8" onsubmit="submitReply(event, {thread_id})">
        <!-- Toolbar -->
        <div class="flex items-center gap-1 mb-2 flex-wrap" id="editorToolbar">
          <button type="button" onclick="insertFormat('**','**')" title="加粗" class="toolbar-btn"><i class="fa fa-bold"></i></button>
          <button type="button" onclick="insertFormat('*','*')" title="斜体" class="toolbar-btn"><i class="fa fa-italic"></i></button>
          <button type="button" onclick="insertFormat('~~','~~')" title="删除线" class="toolbar-btn"><i class="fa fa-strikethrough"></i></button>
          <span class="text-gray-300 mx-1">|</span>
          <button type="button" onclick="insertFormat('`','`')" title="行内代码" class="toolbar-btn"><i class="fa fa-code"></i></button>
          <button type="button" onclick="insertBlock('\n```\n','\n```\n')" title="代码块" class="toolbar-btn"><i class="fa fa-file-code-o"></i></button>
          <span class="text-gray-300 mx-1">|</span>
          <button type="button" onclick="insertBlock('[quote=]','[/quote]')" title="引用" class="toolbar-btn"><i class="fa fa-quote-left"></i></button>
          <button type="button" onclick="insertPrefix('> ')" title="引用块" class="toolbar-btn"><i class="fa fa-indent"></i></button>
          <span class="text-gray-300 mx-1">|</span>
          <button type="button" onclick="insertPrefix('## ')" title="标题" class="toolbar-btn"><i class="fa fa-header"></i></button>
          <button type="button" onclick="insertPrefix('- ')" title="无序列表" class="toolbar-btn"><i class="fa fa-list-ul"></i></button>
          <button type="button" onclick="insertPrefix('1. ')" title="有序列表" class="toolbar-btn"><i class="fa fa-list-ol"></i></button>
          <button type="button" onclick="insertFormat('[','](url)')" title="链接" class="toolbar-btn"><i class="fa fa-link"></i></button>
          <span class="text-gray-300 mx-1">|</span>
          <button type="button" onclick="insertFormat(':smile:','')" title="表情" class="toolbar-btn"><i class="fa fa-smile-o"></i></button>
          <div class="relative" id="emojiPickerWrap">
            <button type="button" onclick="toggleEmojiPicker()" title="表情面板" class="toolbar-btn"><i class="fa fa-heart"></i></button>
            <div id="emojiPicker" class="hidden absolute bottom-full left-0 mb-2 bg-white border border-gray-200 rounded-xl shadow-xl p-3 w-72 z-50">
              <div class="grid grid-cols-8 gap-1 text-xl" id="emojiGrid"></div>
            </div>
          </div>
        </div>
        <textarea name="content" id="replyContent" rows="8" placeholder="支持 Markdown 格式，输入内容..." required
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-y font-mono leading-relaxed"></textarea>
        <div class="flex items-center justify-between mt-3">
          <span class="text-xs text-gray-400">支持 Markdown 格式</span>
          <button type="submit" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
            <i class="fa fa-reply mr-1"></i>发表回复
          </button>
        </div>
      </form>
    </div>"#, thread_id = thread.id),
        Some(_) => r#"<div class="bg-gray-50 rounded-lg p-5 mt-6 text-center text-sm text-gray-400">帖子已关闭，无法回复</div>"#.to_string(),
        None => r#"<div class="bg-gray-50 rounded-lg p-5 mt-6 text-center"><a href="/auth/login" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors inline-block">登录后回复</a></div>"#.to_string(),
    };

    let pagination = pagination_html(page, total_pages, &format!("/thread/{}", thread.id));
    let forum_name = html_escape(thread.forum_name.as_deref().unwrap_or("版块"));

    // Title area action buttons (right side of title)
    let mut title_actions: Vec<String> = Vec::new();
    if let Some(u) = user {
        // Owner: edit (inline)
        if u.id == thread.user_id {
            title_actions.push(format!(r#"<a href="javascript:void(0)" onclick="editPost({first_post_id}, 1)" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-edit"></i> 编辑</a>"#, first_post_id = posts.first().map(|p| p.id).unwrap_or(0)));
            title_actions.push(format!(r#"<a href="javascript:void(0)" onclick="confirmDeleteThread({id})" class="text-xs text-gray-400 hover:text-red-500"><i class="fa fa-trash-o"></i> 删除</a>"#, id = thread.id));
        }
        // Admin: sticky / essence / close / delete
        if u.is_admin() {
            let sticky_text = if thread.is_top == 1 { "取消置顶" } else { "置顶" };
            let essence_text = if thread.is_essence == 1 { "取消精华" } else { "精华" };
            let close_text = if thread.is_closed == 1 { "打开" } else { "关闭" };
            title_actions.push(format!(r#"<a href="javascript:void(0)" onclick="adminToggleSticky({id})" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-thumb-tack"></i> {t}</a>"#, id = thread.id, t = sticky_text));
            title_actions.push(format!(r#"<a href="javascript:void(0)" onclick="adminToggleEssence({id})" class="text-xs text-gray-400 hover:text-orange-500"><i class="fa fa-diamond"></i> {t}</a>"#, id = thread.id, t = essence_text));
            title_actions.push(format!(r#"<a href="javascript:void(0)" onclick="adminToggleClose({id})" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-lock"></i> {t}</a>"#, id = thread.id, t = close_text));
            if u.id != thread.user_id {
                title_actions.push(format!(r#"<a href="javascript:void(0)" onclick="adminDeleteThread({id})" class="text-xs text-red-400 hover:text-red-600"><i class="fa fa-trash-o"></i> 删除</a>"#, id = thread.id));
            }
            title_actions.push(format!(r#"<a href="/admin/thread/{id}/move" class="text-xs text-gray-400 hover:text-blue-600"><i class="fa fa-arrows-alt"></i> 移动</a>"#, id = thread.id));
        }
    }
    let title_actions_html = if title_actions.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="flex items-center gap-4">{}</div>"#, title_actions.join("\n"))
    };

    let sticky_badge = if thread.is_top == 1 { r#"<span class="text-xs bg-red-100 text-red-600 px-2 py-1 rounded font-medium shrink-0">置顶</span>"# } else { "" };
    let essence_badge = if thread.is_essence == 1 { r#"<span class="text-xs bg-orange-100 text-orange-600 px-2 py-1 rounded font-medium shrink-0">精华</span>"# } else { "" };
    let closed_badge = if thread.is_closed == 1 { r#"<span class="text-xs bg-gray-200 text-gray-500 px-2 py-1 rounded font-medium shrink-0">已关闭</span>"# } else { "" };

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/forum/{forum_id}" class="hover:text-black">{forum_name}</a> <i class="fa fa-angle-right"></i> <span class="text-black">{title}</span></div>

    <div class="flex items-start justify-between mb-6 gap-4">
      <div>
        <h1 class="text-2xl font-semibold flex items-center gap-3 flex-wrap">{sticky_badge}{essence_badge}{closed_badge}<span>{title}</span></h1>
        <div class="flex items-center gap-4 mt-2 text-xs text-gray-500">
          <span><i class="fa fa-eye"></i> {views} 查看</span>
          <span><i class="fa fa-comment"></i> {replies} 回复</span>
        </div>
      </div>
      {title_actions}
    </div>

    <div class="bg-white border border-gray-200 rounded-lg overflow-hidden">
      {posts_html}
    </div>

    {pagination}
    {reply_form}"#,
        forum_id = thread.forum_id,
        forum_name = forum_name,
        title = html_escape(&thread.title),
        views = thread.view_count,
        replies = thread.reply_count,
        title_actions = title_actions_html,
        posts_html = posts_html,
        pagination = pagination,
        reply_form = reply_form,
    );

    page_with_sidebar(&thread.title, &main, user, "", "home")
}

// =====================================================================
// New Thread
// =====================================================================

pub fn render_new_thread(forum: &Forum, all_forums: &[Forum], user: &User) -> String {
    let forum_options = all_forums.iter().map(|f| {
        let selected = if f.id == forum.id { " selected" } else { "" };
        format!(r#"<option value="{}"{}>{}</option>"#, f.id, selected, html_escape(&f.name))
    }).collect::<Vec<_>>().join("");

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/forum/{fid}" class="hover:text-black">{fname}</a> <i class="fa fa-angle-right"></i> <span class="text-black">发新帖</span></div>
    <div class="bg-white border border-gray-200 rounded-lg p-6 fade-in">
      <h2 class="text-xl font-semibold mb-6">发布新帖</h2>
      <form id="newThreadForm" accept-charset="UTF-8" onsubmit="submitThread(event)">
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">版块</label>
          <select id="forumSelect" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors bg-white">
            {forum_options}
          </select>
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">标题</label>
          <input type="text" name="title" id="threadTitle" required placeholder="请输入帖子标题"
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">内容</label>
          <!-- Toolbar -->
          <div class="flex items-center gap-1 mb-2 flex-wrap" id="editorToolbar">
            <button type="button" onclick="insertFormatTo('threadContent','**','**')" title="加粗" class="toolbar-btn"><i class="fa fa-bold"></i></button>
            <button type="button" onclick="insertFormatTo('threadContent','*','*')" title="斜体" class="toolbar-btn"><i class="fa fa-italic"></i></button>
            <button type="button" onclick="insertFormatTo('threadContent','~~','~~')" title="删除线" class="toolbar-btn"><i class="fa fa-strikethrough"></i></button>
            <span class="text-gray-300 mx-1">|</span>
            <button type="button" onclick="insertFormatTo('threadContent','`','`')" title="行内代码" class="toolbar-btn"><i class="fa fa-code"></i></button>
            <button type="button" onclick="insertBlockTo('threadContent','\n```\n','\n```\n')" title="代码块" class="toolbar-btn"><i class="fa fa-file-code-o"></i></button>
            <span class="text-gray-300 mx-1">|</span>
            <button type="button" onclick="insertPrefixTo('threadContent','> ')" title="引用块" class="toolbar-btn"><i class="fa fa-indent"></i></button>
            <button type="button" onclick="insertPrefixTo('threadContent','## ')" title="标题" class="toolbar-btn"><i class="fa fa-header"></i></button>
            <button type="button" onclick="insertPrefixTo('threadContent','- ')" title="无序列表" class="toolbar-btn"><i class="fa fa-list-ul"></i></button>
            <button type="button" onclick="insertPrefixTo('threadContent','1. ')" title="有序列表" class="toolbar-btn"><i class="fa fa-list-ol"></i></button>
            <button type="button" onclick="insertFormatTo('threadContent','[','](url)')" title="链接" class="toolbar-btn"><i class="fa fa-link"></i></button>
            <span class="text-gray-300 mx-1">|</span>
            <div class="relative" id="newThreadEmojiWrap">
              <button type="button" onclick="toggleEmojiPickerFor('threadContent','newThreadEmojiGrid','newThreadEmojiPicker')" title="表情" class="toolbar-btn"><i class="fa fa-smile-o"></i></button>
              <div id="newThreadEmojiPicker" class="hidden absolute top-full left-0 mt-2 bg-white border border-gray-200 rounded-xl shadow-xl p-3 w-72 z-50">
                <div class="grid grid-cols-8 gap-1 text-xl" id="newThreadEmojiGrid"></div>
              </div>
            </div>
          </div>
          <textarea name="content" id="threadContent" rows="12" required placeholder="支持 Markdown 格式，输入帖子内容..."
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-y font-mono leading-relaxed"></textarea>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-xs text-gray-400">支持 Markdown 格式</span>
          <div class="flex gap-3">
            <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
              <i class="fa fa-paper-plane mr-1"></i>发布
            </button>
            <a href="/forum/{fid}" class="bg-gray-100 text-black px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors">取消</a>
          </div>
        </div>
      </form>
    </div>"#,
        fid = forum.id,
        fname = html_escape(&forum.name),
        forum_options = forum_options,
    );

    page_with_sidebar("发新帖", &main, Some(user), "", "home")
}

// =====================================================================
// New Thread (generic — no pre-selected forum)
// =====================================================================

pub fn render_new_thread_generic(all_forums: &[Forum], user: &User) -> String {
    let forum_options = all_forums.iter().map(|f| {
        format!(r#"<option value="{}">{}</option>"#, f.id, html_escape(&f.name))
    }).collect::<Vec<_>>().join("");

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <span class="text-black">发新帖</span></div>
    <div class="bg-white border border-gray-200 rounded-lg p-6 fade-in">
      <h2 class="text-xl font-semibold mb-6">发布新帖</h2>
      <form id="newThreadForm" accept-charset="UTF-8" onsubmit="submitThread(event)">
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">版块</label>
          <select id="forumSelect" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors bg-white">
            {forum_options}
          </select>
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">标题</label>
          <input type="text" name="title" id="threadTitle" required placeholder="请输入帖子标题"
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">内容</label>
          <!-- Toolbar -->
          <div class="flex items-center gap-1 mb-2 flex-wrap" id="editorToolbar">
            <button type="button" onclick="insertFormatTo('threadContent','**','**')" title="加粗" class="toolbar-btn"><i class="fa fa-bold"></i></button>
            <button type="button" onclick="insertFormatTo('threadContent','*','*')" title="斜体" class="toolbar-btn"><i class="fa fa-italic"></i></button>
            <button type="button" onclick="insertFormatTo('threadContent','~~','~~')" title="删除线" class="toolbar-btn"><i class="fa fa-strikethrough"></i></button>
            <span class="text-gray-300 mx-1">|</span>
            <button type="button" onclick="insertFormatTo('threadContent','`','`')" title="行内代码" class="toolbar-btn"><i class="fa fa-code"></i></button>
            <button type="button" onclick="insertBlockTo('threadContent','\n```\n','\n```\n')" title="代码块" class="toolbar-btn"><i class="fa fa-file-code-o"></i></button>
            <span class="text-gray-300 mx-1">|</span>
            <button type="button" onclick="insertPrefixTo('threadContent','> ')" title="引用块" class="toolbar-btn"><i class="fa fa-indent"></i></button>
            <button type="button" onclick="insertPrefixTo('threadContent','## ')" title="标题" class="toolbar-btn"><i class="fa fa-header"></i></button>
            <button type="button" onclick="insertPrefixTo('threadContent','- ')" title="无序列表" class="toolbar-btn"><i class="fa fa-list-ul"></i></button>
            <button type="button" onclick="insertPrefixTo('threadContent','1. ')" title="有序列表" class="toolbar-btn"><i class="fa fa-list-ol"></i></button>
            <button type="button" onclick="insertFormatTo('threadContent','[','](url)')" title="链接" class="toolbar-btn"><i class="fa fa-link"></i></button>
            <span class="text-gray-300 mx-1">|</span>
            <div class="relative" id="newThreadEmojiWrap">
              <button type="button" onclick="toggleEmojiPickerFor('threadContent','newThreadEmojiGrid','newThreadEmojiPicker')" title="表情" class="toolbar-btn"><i class="fa fa-smile-o"></i></button>
              <div id="newThreadEmojiPicker" class="hidden absolute top-full left-0 mt-2 bg-white border border-gray-200 rounded-xl shadow-xl p-3 w-72 z-50">
                <div class="grid grid-cols-8 gap-1 text-xl" id="newThreadEmojiGrid"></div>
              </div>
            </div>
          </div>
          <textarea name="content" id="threadContent" rows="12" required placeholder="支持 Markdown 格式，输入帖子内容..."
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-y font-mono leading-relaxed"></textarea>
        </div>
        <div class="flex items-center justify-between">
          <span class="text-xs text-gray-400">支持 Markdown 格式</span>
          <div class="flex gap-3">
            <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
              <i class="fa fa-paper-plane mr-1"></i>发布
            </button>
            <a href="/" class="bg-gray-100 text-black px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors">取消</a>
          </div>
        </div>
      </form>
    </div>"#,
        forum_options = forum_options,
    );

    page_with_sidebar("发新帖", &main, Some(user), "", "home")
}

// =====================================================================
// Profile Page
// =====================================================================

pub fn render_profile(
    user: &User,
    recent_threads: &[ThreadList],
    recent_posts: &[Post],
) -> String {
    let join_date = user.created_at.chars().take(10).collect::<String>();

    let threads_html = if recent_threads.is_empty() {
        r#"<div class="px-5 py-8 text-center text-gray-400 text-sm">暂无发帖</div>"#.to_string()
    } else {
        recent_threads.iter().map(|t| thread_row_html(t)).collect::<Vec<_>>().join("\n")
    };

    let posts_html = if recent_posts.is_empty() {
        r#"<div class="px-5 py-8 text-center text-gray-400 text-sm">暂无回复</div>"#.to_string()
    } else {
        recent_posts.iter().map(|p| {
            let content_preview = if p.content.chars().count() > 80 {
                format!("{}...", html_escape(truncate_chars(&p.content, 80)))
            } else {
                html_escape(&p.content)
            };
            format!(
                r#"<div class="item-hover px-5 py-4 border-b border-gray-100 cursor-pointer" onclick="location.href='/thread/{thread_id}?page={page}#floor-{floor}'">
          <div class="flex items-center justify-between">
            <div class="flex-1 min-w-0">
              <p class="text-sm truncate">{content}</p>
              <div class="flex items-center gap-3 mt-1 text-xs text-gray-500">
                <span>#{floor} 楼</span>
                <span>{time}</span>
              </div>
            </div>
            <i class="fa fa-angle-right text-gray-300 ml-3"></i>
          </div>
        </div>"#,
                thread_id = p.thread_id,
                floor = p.floor,
                page = (p.floor - 1) / 20 + 1,
                content = content_preview,
                time = p.created_at.chars().take(10).collect::<String>(),
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <span class="text-black">个人中心</span></div>

    <!-- Profile header card -->
    <div class="bg-white border border-gray-200 rounded-lg p-6 mb-6 fade-in">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-5">
          {profile_avatar}
          <div>
            <h2 class="text-xl font-semibold">{username}</h2>
            <div class="flex items-center gap-3 mt-1 text-xs text-gray-500">
              <span class="bg-gray-100 px-2 py-0.5 rounded">{group}</span>
              <span><i class="fa fa-calendar"></i> {join_date} 加入</span>
            </div>
          </div>
        </div>
        <a href="/profile/edit" class="bg-black text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors"><i class="fa fa-edit mr-1"></i>编辑资料</a>
      </div>
      <div class="w-full h-px bg-gray-200 my-4"></div>
      <div class="grid grid-cols-4 gap-4 text-center">
        <div><p class="text-lg font-semibold">{threads}</p><p class="text-xs text-gray-500">主题</p></div>
        <div><p class="text-lg font-semibold">{posts}</p><p class="text-xs text-gray-500">帖子</p></div>
        <div><p class="text-lg font-semibold">{credits}</p><p class="text-xs text-gray-500">积分</p></div>
        <div><p class="text-lg font-semibold">{email}</p><p class="text-xs text-gray-500">邮箱</p></div>
      </div>
      {signature}
    </div>

    <!-- Recent threads -->
    <div class="mb-6">
      <div class="flex justify-between items-center mb-3">
        <h3 class="font-semibold">我的主题</h3>
        <a href="/thread/mine" class="text-sm text-gray-500 hover:text-black">查看全部 <i class="fa fa-angle-right"></i></a>
      </div>
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden">
        {threads_html}
      </div>
    </div>

    <!-- Recent posts -->
    <div>
      <h3 class="font-semibold mb-3">最近回复</h3>
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden">
        {posts_html}
      </div>
    </div>"#,
        username = html_escape(&user.username),
        group = user.group_name(),
        join_date = join_date,
        profile_avatar = avatar_html(&user.avatar, user.id, &user.username, "w-16 h-16 text-2xl"),
        threads = user.thread_count,
        posts = user.post_count,
        credits = user.credits,
        email = html_escape(&user.email),
        signature = if user.signature.is_empty() {
            String::new()
        } else {
            format!(r#"<div class="mt-4 p-3 bg-gray-50 rounded-lg text-sm text-gray-600"><i class="fa fa-pencil mr-1"></i> {}</div>"#, html_escape(&user.signature))
        },
        threads_html = threads_html,
        posts_html = posts_html,
    );

    page_with_sidebar("个人中心", &main, Some(user), "", "home")
}

// =====================================================================
// My Threads Page
// =====================================================================

pub fn render_my_threads(
    threads: &[ThreadList],
    page: i64,
    total_pages: i64,
    user: &User,
) -> String {
    let thread_rows = if threads.is_empty() {
        r#"<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无帖子</div>"#.to_string()
    } else {
        threads.iter().map(|t| thread_row_html(t)).collect::<Vec<_>>().join("\n")
    };

    let pagination = pagination_html(page, total_pages, "/thread/mine");

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/profile" class="hover:text-black">个人中心</a> <i class="fa fa-angle-right"></i> <span class="text-black">我的帖子</span></div>
    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-semibold">我的帖子</h2>
      <a href="/new" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors"><i class="fa fa-plus mr-1"></i>发布新帖</a>
    </div>
    <div class="bg-white border border-gray-200 rounded-lg overflow-hidden fade-in">
      {thread_rows}
    </div>
    {pagination}"#,
        thread_rows = thread_rows,
        pagination = pagination,
    );

    page_with_sidebar("我的帖子", &main, Some(user), "", "home")
}

// =====================================================================
// Auth Pages (no sidebar — centered layout)
// =====================================================================

pub fn render_login() -> String {
    let content = r#"
<div class="container mx-auto px-4 py-16 max-w-md">
  <div class="bg-white border border-gray-200 rounded-lg p-8 fade-in">
    <h2 class="text-2xl font-semibold text-center mb-8">登录</h2>
    <form id="loginForm" accept-charset="UTF-8" onsubmit="submitLogin(event)">
      <div class="mb-5">
        <label class="block text-sm font-medium mb-2">用户名</label>
        <input type="text" name="username" required placeholder="请输入用户名"
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>
      <div class="mb-6">
        <label class="block text-sm font-medium mb-2">密码</label>
        <input type="password" name="password" required placeholder="请输入密码"
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>
      <button type="submit" class="w-full bg-black text-white py-3 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">登录</button>
    </form>
    <p class="text-center text-sm text-gray-500 mt-6">还没有账号？<a href="/auth/register" class="text-black font-medium hover:underline">立即注册</a></p>
  </div>
</div>"#.to_string();
    layout("登录", None, &content, "home")
}

pub fn render_register(allow_register: bool, invite_required: bool) -> String {
    if !allow_register {
        let content = r#"
<div class="container mx-auto px-4 py-16 max-w-md text-center">
  <div class="bg-white border border-gray-200 rounded-lg p-8 fade-in">
    <i class="fa fa-lock text-4xl text-gray-300 mb-4"></i>
    <p class="text-lg mb-6">注册功能已关闭</p>
    <a href="/auth/login" class="text-black font-medium hover:underline">返回登录</a>
  </div>
</div>"#.to_string();
        return layout("注册", None, &content, "home");
    }
    let invite_field = if invite_required {
        r#"<div class="mb-5">
        <label class="block text-sm font-medium mb-2">邀请码</label>
        <input type="text" name="invite_code" required placeholder="请输入邀请码"
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>"#
    } else {
        ""
    };
    let content = format!(r#"
<div class="container mx-auto px-4 py-16 max-w-md">
  <div class="bg-white border border-gray-200 rounded-lg p-8 fade-in">
    <h2 class="text-2xl font-semibold text-center mb-8">注册</h2>
    <form id="registerForm" accept-charset="UTF-8" onsubmit="submitRegister(event)">
      <div class="mb-5">
        <label class="block text-sm font-medium mb-2">用户名</label>
        <input type="text" name="username" required placeholder="请输入用户名"
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>
      <div class="mb-5">
        <label class="block text-sm font-medium mb-2">邮箱</label>
        <input type="email" name="email" required placeholder="请输入邮箱"
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>
      {invite_field}
      <div class="mb-5">
        <label class="block text-sm font-medium mb-2">密码</label>
        <input type="password" name="password" required placeholder="至少6位" minlength="6"
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>
      <div class="mb-6">
        <label class="block text-sm font-medium mb-2">确认密码</label>
        <input type="password" name="password_confirm" required placeholder="再次输入密码"
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>
      <button type="submit" class="w-full bg-black text-white py-3 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">注册</button>
    </form>
    <p class="text-center text-sm text-gray-500 mt-6">已有账号？<a href="/auth/login" class="text-black font-medium hover:underline">立即登录</a></p>
  </div>
</div>"#,
        invite_field = invite_field,
    );
    layout("注册", None, &content, "home")
}

pub fn render_error(msg: &str) -> String {
    let content = format!(r#"
<div class="container mx-auto px-4 py-16 max-w-md text-center">
  <div class="bg-white border border-gray-200 rounded-lg p-8 fade-in">
    <i class="fa fa-exclamation-circle text-4xl text-gray-300 mb-4"></i>
    <p class="text-lg mb-6">{msg}</p>
    <a href="/" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors inline-block">返回首页</a>
  </div>
</div>"#,
        msg = html_escape(msg),
    );
    layout("错误", None, &content, "home")
}

// 通用消息页面（用于邮箱验证提示等）
pub fn render_message_page(title: &str, body_html: &str) -> String {
    let content = format!(r#"
<div class="container mx-auto px-4 py-16 max-w-md text-center">
  <div class="bg-white border border-gray-200 rounded-lg p-8 fade-in">
    <i class="fa fa-envelope text-4xl text-gray-300 mb-4"></i>
    <h2 class="text-lg font-semibold mb-4">{title}</h2>
    <div class="text-sm text-gray-600 mb-6">{body}</div>
    <a href="/" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors inline-block">返回首页</a>
  </div>
</div>"#,
        title = html_escape(title),
        body = body_html,
    );
    layout(title, None, &content, "home")
}

// =====================================================================
// Admin Pages — Independent Layout
// =====================================================================

pub fn admin_layout(title: &str, active: &str, content: &str) -> String {
    let menu_items = [
        ("dashboard", "fa-tachometer", "仪表盘", "/admin"),
        ("threads", "fa-files-o", "帖子管理", "/admin/threads"),
        ("forums", "fa-th-large", "版块管理", "/admin/forums"),
        ("users", "fa-users", "用户管理", "/admin/users"),
        ("invite", "fa-ticket", "邀请码管理", "/admin/invite-codes"),
        ("reports", "fa-shield", "举报管理", "/admin/reports"),
        ("blacklist", "fa-ban", "黑名单管理", "/admin/blacklist"),
        ("login-logs", "fa-history", "登录日志", "/admin/login-logs"),
        ("review", "fa-search", "安全审查", "/admin/review"),
        ("backup", "fa-database", "数据备份", "/admin/backup"),
        ("settings", "fa-cog", "系统设置", "/admin/settings/site"),
    ];

    let menu_html = menu_items.iter().map(|(key, icon, label, href)| {
        let cls = if *key == active || (active.starts_with("settings") && *key == "settings") {
            "bg-gray-700 text-white"
        } else {
            "text-gray-300 hover:bg-gray-700 hover:text-white"
        };
        format!(r#"<a href="{href}" class="flex items-center gap-3 px-4 py-2.5 rounded-lg text-sm transition-colors {cls}"><i class="fa {icon} w-5 text-center"></i>{label}</a>"#)
    }).collect::<Vec<_>>().join("\n");

    let settings_sub = if active.starts_with("settings") {
        let subs = [
            ("site", "站点信息", "/admin/settings/site"),
            ("register", "注册设置", "/admin/settings/register"),
            ("credits", "积分设置", "/admin/settings/credits"),
            ("upload", "上传设置", "/admin/settings/upload"),
            ("ai", "AI 审查设置", "/admin/settings/ai"),
            ("email", "邮件设置", "/admin/settings/email"),
        ];
        let sub_html = subs.iter().map(|(key, label, href)| {
            let cls = if active == *key { "bg-gray-600 text-white" } else { "text-gray-400 hover:text-white" };
            format!(r#"<a href="{href}" class="block pl-12 pr-4 py-2 text-xs rounded transition-colors {cls}">{label}</a>"#)
        }).collect::<Vec<_>>().join("\n");
        format!(r#"<div class="mt-1 space-y-0.5">{sub_html}</div>"#)
    } else {
        String::new()
    };

    format!(r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{title} | 管理后台</title>
  <script src="/static/css/tailwind.js"></script>
  <link href="/static/css/font-awesome.min.css" rel="stylesheet">
  <script>
    tailwind.config = {{
      theme: {{
        extend: {{
          colors: {{
            primary: '#000000',
            secondary: '#666666',
          }},
        }},
      }}
    }}
  </script>
  <style>
    .fade-in {{ animation: fadeIn 0.3s ease; }}
    @keyframes fadeIn {{ from {{ opacity:0; transform:translateY(8px) }} to {{ opacity:1; transform:translateY(0) }} }}
    .item-hover {{ transition: all 0.2s ease; }}
    .item-hover:hover {{ background: #f9f9f9; }}
  </style>
</head>
<body class="bg-gray-100 text-black min-h-screen">
<div class="flex min-h-screen">
  <!-- Sidebar -->
  <aside class="w-60 bg-gray-800 flex-shrink-0 flex flex-col">
    <div class="px-5 py-5 border-b border-gray-700">
      <a href="/admin" class="flex items-center gap-2 text-white font-bold text-lg">
        <i class="fa fa-shield"></i> 管理后台
      </a>
    </div>
    <nav class="flex-1 px-3 py-4 space-y-1 overflow-y-auto">
      {menu_html}
      {settings_sub}
    </nav>
    <div class="px-3 py-4 border-t border-gray-700 space-y-1">
      <a href="/" class="flex items-center gap-3 px-4 py-2.5 rounded-lg text-sm text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"><i class="fa fa-external-link w-5 text-center"></i>返回前台</a>
      <a href="/auth/logout" class="flex items-center gap-3 px-4 py-2.5 rounded-lg text-sm text-gray-400 hover:bg-gray-700 hover:text-white transition-colors"><i class="fa fa-sign-out w-5 text-center"></i>退出登录</a>
    </div>
  </aside>
  <!-- Main content -->
  <main class="flex-1 overflow-y-auto">
    <div class="p-6 lg:p-8">
      {content}
    </div>
  </main>
</div>
<div id="toast" class="fixed top-6 right-6 z-[999] hidden">
  <div class="bg-black text-white px-5 py-3 rounded-lg shadow-lg text-sm" id="toastMsg"></div>
</div>
<script src="/static/js/app.js"></script>
<script>
(function(){{
  var params = new URLSearchParams(window.location.search);
  var msg = params.get('saved');
  var err = params.get('error');
  if (msg === '1') {{ showToast('保存成功'); }}
  else if (err) {{ showToast('操作失败：' + decodeURIComponent(err)); }}
  // Clean URL without reloading
  if (msg || err) {{
    var url = new URL(window.location);
    url.searchParams.delete('saved');
    url.searchParams.delete('error');
    window.history.replaceState({{}}, '', url.toString());
  }}
}})();
</script>
</body>
</html>"##,
        title = html_escape(title),
        menu_html = menu_html,
        settings_sub = settings_sub,
        content = content,
    )
}

// ------------------------------------------------------------------
// Dashboard
// ------------------------------------------------------------------

pub fn render_admin_dashboard(stats: &SiteStats, pending_reports: i64, today_checkins: i64, today_users: i64, blacklist_count: i64, recent_reports: &[ReportWithReporter], recent_users: &[User]) -> String {
    let cards = [
        (stats.total_users, "用户总数", "fa-users", "bg-blue-50 text-blue-600"),
        (stats.total_threads, "主题总数", "fa-file-text-o", "bg-green-50 text-green-600"),
        (stats.total_posts, "帖子总数", "fa-comment-o", "bg-purple-50 text-purple-600"),
        (stats.total_forums, "版块数", "fa-th-large", "bg-orange-50 text-orange-600"),
        (today_checkins, "今日签到", "fa-calendar-check-o", "bg-teal-50 text-teal-600"),
        (pending_reports, "待处理举报", "fa-shield", "bg-red-50 text-red-600"),
        (today_users, "今日新注册", "fa-user-plus", "bg-indigo-50 text-indigo-600"),
        (blacklist_count, "黑名单数", "fa-ban", "bg-gray-100 text-gray-600"),
    ];
    let cards_html = cards.iter().map(|(count, label, icon, color)| {
        format!(r#"<div class="bg-white rounded-xl p-5 border border-gray-100 shadow-sm fade-in">
      <div class="flex items-center gap-4">
        <div class="w-12 h-12 rounded-xl {color} flex items-center justify-center"><i class="fa {icon} text-xl"></i></div>
        <div><p class="text-2xl font-bold">{count}</p><p class="text-gray-500 text-xs mt-0.5">{label}</p></div>
      </div>
    </div>"#)
    }).collect::<Vec<_>>().join("\n");

    let reports_html = if recent_reports.is_empty() {
        r#"<div class="text-center text-gray-400 text-sm py-8">暂无举报</div>"#.to_string()
    } else {
        recent_reports.iter().map(|r| {
            let status_cls = match r.status.as_str() {
                "pending" => "bg-yellow-100 text-yellow-700",
                "reviewing" => "bg-blue-100 text-blue-700",
                "resolved" => "bg-green-100 text-green-700",
                _ => "bg-gray-100 text-gray-600",
            };
            let status_text = match r.status.as_str() {
                "pending" => "待处理",
                "reviewing" => "处理中",
                "resolved" => "已处理",
                _ => "已驳回",
            };
            format!(r#"<tr class="border-b border-gray-50 item-hover">
          <td class="px-4 py-3 text-sm">{reporter}</td>
          <td class="px-4 py-3 text-sm">{target_type}</td>
          <td class="px-4 py-3 text-sm text-gray-500">{reason}</td>
          <td class="px-4 py-3 text-sm"><span class="px-2 py-0.5 rounded text-xs {status_cls}">{status_text}</span></td>
          <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        </tr>"#,
                reporter = html_escape(&r.reporter_name),
                target_type = match r.target_type.as_str() { "thread" => "主题", "post" => "回复", _ => "用户" },
                reason = html_escape(truncate_chars(&r.reason, 20)),
                status_cls = status_cls, status_text = status_text,
                time = r.created_at.chars().take(16).collect::<String>(),
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let users_html = if recent_users.is_empty() {
        r#"<div class="text-center text-gray-400 text-sm py-8">暂无</div>"#.to_string()
    } else {
        recent_users.iter().map(|u| {
            format!(r#"<tr class="border-b border-gray-50 item-hover">
          <td class="px-4 py-3 text-sm font-medium">{username}</td>
          <td class="px-4 py-3 text-sm text-gray-500">{group}</td>
          <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        </tr>"#,
                username = html_escape(&u.username),
                group = u.group_name(),
                time = u.created_at.chars().take(10).collect::<String>(),
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">仪表盘</h1>
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">{cards_html}</div>
  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <div class="bg-white rounded-xl border border-gray-100 shadow-sm">
      <div class="px-5 py-4 border-b border-gray-100 flex justify-between items-center">
        <h2 class="font-semibold">最近举报</h2>
        <a href="/admin/reports" class="text-xs text-blue-600 hover:underline">查看全部</a>
      </div>
      <table class="w-full"><tbody>{reports_html}</tbody></table>
    </div>
    <div class="bg-white rounded-xl border border-gray-100 shadow-sm">
      <div class="px-5 py-4 border-b border-gray-100 flex justify-between items-center">
        <h2 class="font-semibold">最近注册</h2>
        <a href="/admin/users" class="text-xs text-blue-600 hover:underline">查看全部</a>
      </div>
      <table class="w-full"><tbody>{users_html}</tbody></table>
    </div>
  </div>"#);
    admin_layout("仪表盘", "dashboard", &content)
}

// ------------------------------------------------------------------
// Thread Management
// ------------------------------------------------------------------

pub fn render_admin_threads(threads: &[AdminThreadRow], page: i64, total_pages: i64) -> String {
    let rows = threads.iter().map(|t| {
        let mut badges = String::new();
        if t.is_top == 1 { badges.push_str(r#"<span class="bg-red-100 text-red-600 text-xs px-1.5 py-0.5 rounded ml-1">置顶</span>"#); }
        if t.is_essence == 1 { badges.push_str(r#"<span class="bg-yellow-100 text-yellow-700 text-xs px-1.5 py-0.5 rounded ml-1">精华</span>"#); }
        if t.is_closed == 1 { badges.push_str(r#"<span class="bg-gray-200 text-gray-600 text-xs px-1.5 py-0.5 rounded ml-1">已关闭</span>"#); }
        format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm text-gray-400">{id}</td>
        <td class="px-4 py-3 text-sm"><a href="/thread/{id}" class="font-medium hover:text-blue-600">{title}</a>{badges}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{author}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{forum}</td>
        <td class="px-4 py-3 text-sm text-center">{replies}</td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        <td class="px-4 py-3 text-sm space-x-1">
          <button onclick="postAction('/admin/thread/{id}/sticky')" class="text-xs text-blue-600 hover:underline">置顶</button>
          <button onclick="postAction('/admin/thread/{id}/essence')" class="text-xs text-yellow-600 hover:underline">精华</button>
          <button onclick="postAction('/admin/thread/{id}/close')" class="text-xs text-gray-600 hover:underline">关闭</button>
          <a href="/admin/thread/{id}/move" class="text-xs text-green-600 hover:underline">移动</a>
          <button onclick="if(confirm('确定删除？'))postAction('/admin/thread/{id}/delete')" class="text-xs text-red-500 hover:underline">删除</button>
        </td>
      </tr>"#,
            id = t.id,
            title = html_escape(&t.title),
            badges = badges,
            author = html_escape(&t.author_name),
            forum = html_escape(&t.forum_name),
            replies = t.reply_count,
            time = t.created_at.chars().take(10).collect::<String>(),
        )
    }).collect::<Vec<_>>().join("\n");

    let pag = pagination_html(page, total_pages, "/admin/threads");
    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">帖子管理</h1>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden fade-in">
    <table class="w-full">
      <thead class="bg-gray-50 text-xs text-gray-500"><tr>
        <th class="px-4 py-3 text-left w-16">ID</th><th class="px-4 py-3 text-left">标题</th><th class="px-4 py-3 text-left">作者</th><th class="px-4 py-3 text-left">版块</th><th class="px-4 py-3 text-center w-16">回复</th><th class="px-4 py-3 text-left w-24">时间</th><th class="px-4 py-3 text-left w-48">操作</th>
      </tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
  {pag}"#);
    admin_layout("帖子管理", "threads", &content)
}

// ------------------------------------------------------------------
// Move Thread
// ------------------------------------------------------------------

pub fn render_move_thread(thread: &Thread, current_forum: &Forum, forums: &[Forum]) -> String {
    let options = forums.iter().filter(|f| f.id != current_forum.id).map(|f| {
        format!(r#"<option value="{id}">{name}</option>"#, id = f.id, name = html_escape(&f.name))
    }).collect::<Vec<_>>().join("\n");

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">移动帖子</h1>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 fade-in max-w-lg">
    <div class="mb-5">
      <div class="text-sm text-gray-500 mb-1">帖子标题</div>
      <div class="font-medium text-lg">{title}</div>
    </div>
    <div class="mb-5">
      <div class="text-sm text-gray-500 mb-1">当前版块</div>
      <div class="font-medium">{forum}</div>
    </div>
    <form method="POST" action="/admin/thread/{thread_id}/move" accept-charset="UTF-8">
      <div class="mb-5">
        <label class="text-sm text-gray-500 block mb-1">目标版块</label>
        <select name="target_forum_id" required class="w-full border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
          {options}
        </select>
      </div>
      <div class="flex gap-3">
        <button type="submit" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800"><i class="fa fa-arrows-alt mr-1"></i>确认移动</button>
        <a href="/thread/{thread_id}" class="bg-gray-100 text-gray-600 px-5 py-2 rounded-lg text-sm hover:bg-gray-200">取消</a>
      </div>
    </form>
  </div>"#,
        title = html_escape(&thread.title),
        forum = html_escape(&current_forum.name),
        thread_id = thread.id,
        options = options,
    );
    admin_layout("移动帖子", "threads", &content)
}

// ------------------------------------------------------------------
// Forum Management (card layout)
// ------------------------------------------------------------------

pub fn render_admin_forums(forums: &[Forum], moderators: &[crate::models::forum_moderator::ForumModeratorWithUser]) -> String {
    let cards = forums.iter().map(|f| {
        let status_badge = if f.status == 1 {
            r#"<span class="bg-green-100 text-green-700 text-xs px-2 py-0.5 rounded">正常</span>"#
        } else {
            r#"<span class="bg-red-100 text-red-600 text-xs px-2 py-0.5 rounded">隐藏</span>"#
        };

        // 该版块的版主列表
        let forum_mods: Vec<_> = moderators.iter().filter(|m| m.forum_id == f.id).collect();
        let mods_html = if forum_mods.is_empty() {
            r#"<span class="text-gray-400 text-xs">暂无版主</span>"#.to_string()
        } else {
            let items = forum_mods.iter().map(|m| {
                format!(r#"<span class="bg-blue-50 text-blue-600 text-xs px-2 py-0.5 rounded inline-flex items-center gap-1">{} <a href="/admin/forums/{fid}/moderators/{uid}/remove" onclick="return confirm('确定移除版主？')" class="text-red-400 hover:text-red-600">&times;</a></span>"#,
                    html_escape(&m.username), fid = f.id, uid = m.user_id)
            }).collect::<Vec<_>>().join(" ");
            items
        };

        format!(r#"<div class="bg-white rounded-xl border border-gray-100 shadow-sm p-5 fade-in">
      <div class="flex items-start justify-between mb-3">
        <div>
          <h3 class="font-semibold text-lg">{name}</h3>
          <p class="text-gray-500 text-sm mt-1">{desc}</p>
        </div>
        {status_badge}
      </div>
      <div class="flex items-center gap-4 text-xs text-gray-500 mb-4">
        <span><i class="fa fa-file-text-o mr-1"></i>{threads} 主题</span>
        <span><i class="fa fa-comment-o mr-1"></i>{posts} 帖子</span>
        <span><i class="fa fa-sort mr-1"></i>排序 {sort}</span>
      </div>
      <form id="forum-{id}" method="POST" action="/admin/forums/{id}/edit" accept-charset="UTF-8" class="grid grid-cols-2 gap-2 mb-3">
        <input name="name" value="{name}" class="border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
        <input name="description" value="{desc}" class="border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
        <input name="sort_order" type="number" value="{sort}" class="border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
        <select name="status" class="border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
          <option value="1" {sel1}>正常</option><option value="0" {sel0}>隐藏</option>
        </select>
        <select name="view_perm" class="border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
          <option value="0" {vp0}>浏览: 所有人</option><option value="1" {vp1}>浏览: 登录用户</option><option value="2" {vp2}>浏览: 版主+</option><option value="3" {vp3}>浏览: 仅管理员</option>
        </select>
        <select name="post_perm" class="border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
          <option value="0" {pp0}>发帖: 所有用户</option><option value="1" {pp1}>发帖: 版主+</option><option value="2" {pp2}>发帖: 仅管理员</option>
        </select>
        <div class="flex gap-2 col-span-2">
          <button type="submit" class="bg-black text-white px-3 py-1.5 rounded-lg text-xs hover:bg-gray-800"><i class="fa fa-save mr-1"></i>保存</button>
          <a href="/admin/forums/{id}/delete" onclick="return confirm('确定删除？所有帖子将被删除！')" class="bg-gray-100 text-red-500 px-3 py-1.5 rounded-lg text-xs hover:bg-gray-200"><i class="fa fa-trash-o mr-1"></i>删除</a>
        </div>
      </form>
      <div class="border-t border-gray-100 pt-3 mt-2">
        <div class="text-xs text-gray-500 mb-2"><i class="fa fa-user-shield mr-1"></i>版主管理</div>
        <div class="flex flex-wrap gap-1 mb-2">{mods_html}</div>
        <form method="POST" action="/admin/forums/{id}/moderators/add" class="flex gap-1">
          <input name="user_id" type="number" placeholder="用户ID" class="border border-gray-200 rounded px-2 py-1 text-xs w-20 outline-none focus:border-black">
          <button type="submit" class="bg-blue-500 text-white px-2 py-1 rounded text-xs hover:bg-blue-600">添加版主</button>
        </form>
      </div>
    </div>"#,
            id = f.id,
            name = html_escape(&f.name),
            desc = html_escape(&f.description),
            threads = f.thread_count,
            posts = f.post_count,
            sort = f.sort_order,
            sel1 = if f.status == 1 { "selected" } else { "" },
            sel0 = if f.status == 0 { "selected" } else { "" },
            vp0 = if f.view_perm == 0 { "selected" } else { "" },
            vp1 = if f.view_perm == 1 { "selected" } else { "" },
            vp2 = if f.view_perm == 2 { "selected" } else { "" },
            vp3 = if f.view_perm == 3 { "selected" } else { "" },
            pp0 = if f.post_perm == 0 { "selected" } else { "" },
            pp1 = if f.post_perm == 1 { "selected" } else { "" },
            pp2 = if f.post_perm == 2 { "selected" } else { "" },
            mods_html = mods_html,
        )
    }).collect::<Vec<_>>().join("\n");

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">版块管理</h1>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-5 mb-6 fade-in">
    <h3 class="font-semibold mb-4"><i class="fa fa-plus-circle mr-1"></i>添加版块</h3>
    <form method="POST" action="/admin/forums/create" accept-charset="UTF-8" class="flex gap-3 items-end">
      <div class="flex-1"><label class="text-xs text-gray-500">名称</label><input name="name" required class="w-full border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black"></div>
      <div class="flex-1"><label class="text-xs text-gray-500">描述</label><input name="description" class="w-full border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black"></div>
      <div class="w-24"><label class="text-xs text-gray-500">排序</label><input name="sort_order" type="number" value="0" class="w-full border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black"></div>
      <button type="submit" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800"><i class="fa fa-plus mr-1"></i>添加</button>
    </form>
  </div>
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">{cards}</div>"#);
    admin_layout("版块管理", "forums", &content)
}

// ------------------------------------------------------------------
// User Management
// ------------------------------------------------------------------

pub fn render_admin_users(users: &[User], muted_ids: &[i64]) -> String {
    let rows = users.iter().map(|u| {
        let status_text = if u.status == 1 { "正常" } else { "封禁" };
        let status_cls = if u.status == 1 { "text-green-600" } else { "text-red-500" };
        let toggle_text = if u.status == 1 { "封禁" } else { "解封" };
        let toggle_cls = if u.status == 1 { "text-red-500" } else { "text-green-600" };
        let muted_badge = if muted_ids.contains(&u.id) {
            r#"<span class="bg-orange-100 text-orange-600 text-xs px-1.5 py-0.5 rounded ml-1">禁言</span>"#
        } else {
            ""
        };
        let unmute_link = if muted_ids.contains(&u.id) {
            format!(r#"<a href="/admin/users/{id}/unmute" class="text-xs text-green-600 hover:underline">解禁</a>"#, id = u.id)
        } else {
            String::new()
        };
        format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm text-gray-400">{id}</td>
        <td class="px-4 py-3 text-sm font-medium">{username}{muted_badge}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{email}</td>
        <td class="px-4 py-3 text-sm">{group}</td>
        <td class="px-4 py-3 text-sm {status_cls}">{status_text}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{credits} / {posts} / {threads}</td>
        <td class="px-4 py-3 text-xs text-gray-400 font-mono">{last_ip}</td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        <td class="px-4 py-3 text-sm space-x-1">
          <select onchange="location.href=this.value" class="text-xs border rounded px-1 py-0.5">
            <option>设为...</option>
            <option value="/admin/users/{id}/group/1">管理员</option>
            <option value="/admin/users/{id}/group/2">版主</option>
            <option value="/admin/users/{id}/group/3">会员</option>
          </select>
          <a href="/admin/users/{id}/toggle" class="text-xs {toggle_cls} hover:underline">{toggle_text}</a>
          <button onclick="showMuteDialog({id},'{username}')" class="text-xs text-orange-600 hover:underline">禁言</button>
          {unmute_link}
        </td>
      </tr>"#,
            id = u.id,
            username = html_escape(&u.username),
            muted_badge = muted_badge,
            unmute_link = unmute_link,
            email = html_escape(&u.email),
            group = u.group_name(),
            status_cls = status_cls,
            status_text = status_text,
            credits = u.credits,
            posts = u.post_count,
            threads = u.thread_count,
            time = u.created_at.chars().take(10).collect::<String>(),
            last_ip = if u.last_login_ip.is_empty() { "-".to_string() } else { html_escape(&u.last_login_ip) },
            toggle_cls = toggle_cls,
            toggle_text = toggle_text,
        )
    }).collect::<Vec<_>>().join("\n");

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">用户管理</h1>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden fade-in">
    <table class="w-full">
      <thead class="bg-gray-50 text-xs text-gray-500"><tr>
        <th class="px-4 py-3 text-left w-12">ID</th><th class="px-4 py-3 text-left">用户名</th><th class="px-4 py-3 text-left">邮箱</th><th class="px-4 py-3 text-left">用户组</th><th class="px-4 py-3 text-left">状态</th><th class="px-4 py-3 text-left">积分/帖/主</th><th class="px-4 py-3 text-left">最近IP</th><th class="px-4 py-3 text-left w-24">注册</th><th class="px-4 py-3 text-left w-56">操作</th>
      </tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
  <!-- Mute dialog -->
  <div id="muteDialog" class="fixed inset-0 bg-black bg-opacity-50 z-50 hidden flex items-center justify-center">
    <div class="bg-white rounded-xl p-6 w-96 shadow-xl">
      <h3 class="font-semibold text-lg mb-4">禁言用户: <span id="muteUsername"></span></h3>
      <form id="muteForm" method="POST" accept-charset="UTF-8">
        <input type="hidden" id="muteUserId" name="user_id">
        <div class="mb-3"><label class="text-sm text-gray-600">禁言天数</label>
          <select name="days" class="w-full border rounded-lg px-3 py-2 text-sm mt-1">
            <option value="1">1 天</option><option value="3">3 天</option><option value="7" selected>7 天</option>
            <option value="30">30 天</option><option value="365">365 天</option><option value="0">永久</option>
          </select>
        </div>
        <div class="mb-4"><label class="text-sm text-gray-600">原因</label>
          <textarea name="reason" rows="2" class="w-full border rounded-lg px-3 py-2 text-sm mt-1" placeholder="禁言原因..."></textarea>
        </div>
        <div class="flex gap-2">
          <button type="submit" class="bg-orange-500 text-white px-4 py-2 rounded-lg text-sm hover:bg-orange-600">确认禁言</button>
          <button type="button" onclick="document.getElementById('muteDialog').classList.add('hidden')" class="bg-gray-100 px-4 py-2 rounded-lg text-sm">取消</button>
        </div>
      </form>
    </div>
  </div>
  <script>
  function showMuteDialog(uid, uname) {{
    document.getElementById('muteUserId').value = uid;
    document.getElementById('muteUsername').textContent = uname;
    document.getElementById('muteForm').action = '/admin/users/' + uid + '/mute';
    document.getElementById('muteDialog').classList.remove('hidden');
  }}
  </script>"#);
    admin_layout("用户管理", "users", &content)
}

// ------------------------------------------------------------------
// Reports Management
// ------------------------------------------------------------------

pub fn render_admin_reports(reports: &[ReportWithReporter], status_filter: &str, counts: (i64, i64, i64, i64)) -> String {
    let (pending, reviewing, resolved, dismissed) = counts;
    let tabs = [
        ("all", "全部", None),
        ("pending", "待处理", Some(pending)),
        ("reviewing", "处理中", Some(reviewing)),
        ("resolved", "已处理", Some(resolved)),
        ("dismissed", "已驳回", Some(dismissed)),
    ];
    let tabs_html = tabs.iter().map(|(key, label, count)| {
        let cls = if *key == status_filter { "bg-black text-white" } else { "bg-gray-100 text-gray-600 hover:bg-gray-200" };
        let badge = count.map(|c| format!(r#"<span class="ml-1 text-xs">({})</span>"#, c)).unwrap_or_default();
        format!(r#"<a href="/admin/reports?status={key}" class="px-4 py-2 rounded-lg text-sm font-medium {cls}">{label}{badge}</a>"#)
    }).collect::<Vec<_>>().join("\n");

    let rows = if reports.is_empty() {
        r#"<tr><td colspan="6" class="px-4 py-12 text-center text-gray-400 text-sm">暂无举报记录</td></tr>"#.to_string()
    } else {
        reports.iter().map(|r| {
            let status_cls = match r.status.as_str() {
                "pending" => "bg-yellow-100 text-yellow-700",
                "reviewing" => "bg-blue-100 text-blue-700",
                "resolved" => "bg-green-100 text-green-700",
                _ => "bg-gray-100 text-gray-600",
            };
            let status_text = match r.status.as_str() {
                "pending" => "待处理",
                "reviewing" => "处理中",
                "resolved" => "已处理",
                _ => "已驳回",
            };
            let target_link = match r.target_type.as_str() {
                "thread" => format!(r#"<a href="/thread/{}" class="text-blue-600 hover:underline" target="_blank">主题</a>"#, r.target_id),
                "post" => format!(r#"<a href="/thread/{}" class="text-blue-600 hover:underline" target="_blank">回复</a>"#, r.target_id),
                _ => format!(r#"<a href="/user/{}" class="text-blue-600 hover:underline" target="_blank">用户</a>"#, r.target_id),
            };
            let actions = if r.status == "pending" || r.status == "reviewing" {
                format!(r#"<div class="flex gap-1 mt-2">
              <button onclick="handleReport({id},'resolve')" class="bg-green-500 text-white px-2 py-1 rounded text-xs">处理完成</button>
              <button onclick="handleReport({id},'dismiss')" class="bg-gray-300 text-gray-700 px-2 py-1 rounded text-xs">驳回</button>
            </div>
            <div class="mt-2"><input id="note{id}" placeholder="管理员备注" class="border rounded px-2 py-1 text-xs w-full"></div>"#,
                    id = r.id)
            } else {
                String::new()
            };
            format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm">{reporter}</td>
        <td class="px-4 py-3 text-sm">{target_link}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{reason}</td>
        <td class="px-4 py-3 text-sm"><span class="px-2 py-0.5 rounded text-xs {status_cls}">{status_text}</span></td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        <td class="px-4 py-3 text-sm">{actions}</td>
      </tr>"#,
                reporter = html_escape(&r.reporter_name),
                target_link = target_link,
                reason = html_escape(&r.reason),
                status_cls = status_cls,
                status_text = status_text,
                time = r.created_at.chars().take(16).collect::<String>(),
                actions = actions,
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">举报管理</h1>
  <div class="flex gap-2 mb-6">{tabs_html}</div>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden fade-in">
    <table class="w-full">
      <thead class="bg-gray-50 text-xs text-gray-500"><tr>
        <th class="px-4 py-3 text-left">举报人</th><th class="px-4 py-3 text-left">类型</th><th class="px-4 py-3 text-left">原因</th><th class="px-4 py-3 text-left">状态</th><th class="px-4 py-3 text-left w-32">时间</th><th class="px-4 py-3 text-left">操作</th>
      </tr></thead>
      <tbody>{rows}</tbody>
    </table>
  </div>
  <script>
  function handleReport(id, action) {{
    var note = document.getElementById('note'+id) ? document.getElementById('note'+id).value : '';
    fetch('/admin/reports/'+id+'/action', {{
      method:'POST', headers:{{'Content-Type':'application/x-www-form-urlencoded'}},
      body:'action='+action+'&note='+encodeURIComponent(note)
    }}).then(function(){{ location.reload(); }});
  }}
  </script>"#);
    admin_layout("举报管理", "reports", &content)
}

// ------------------------------------------------------------------
// Blacklist Management
// ------------------------------------------------------------------

pub fn render_admin_blacklist(entries: &[BlacklistEntry], muted_users: &[MutedUserWithInfo]) -> String {
    let ip_rows = entries.iter().filter(|e| e.r#type == "ip").map(|e| {
        format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm font-mono">{value}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{reason}</td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        <td class="px-4 py-3 text-sm"><button onclick="if(confirm('确定移除？'))postAction('/admin/blacklist/{id}/delete')" class="text-xs text-red-500 hover:underline">移除</button></td>
      </tr>"#,
            value = html_escape(&e.value),
            reason = html_escape(&e.reason),
            time = e.created_at.chars().take(16).collect::<String>(),
            id = e.id,
        )
    }).collect::<Vec<_>>().join("\n");

    let user_rows = entries.iter().filter(|e| e.r#type == "user").map(|e| {
        format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm">{value}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{reason}</td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        <td class="px-4 py-3 text-sm"><button onclick="if(confirm('确定移除？'))postAction('/admin/blacklist/{id}/delete')" class="text-xs text-red-500 hover:underline">移除</button></td>
      </tr>"#,
            value = html_escape(&e.value),
            reason = html_escape(&e.reason),
            time = e.created_at.chars().take(16).collect::<String>(),
            id = e.id,
        )
    }).collect::<Vec<_>>().join("\n");

    let muted_rows = muted_users.iter().map(|m| {
        let expires_text = match &m.expires_at {
            Some(t) => t.chars().take(16).collect::<String>(),
            None => "永久".to_string(),
        };
        format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm font-medium">{username}</td>
        <td class="px-4 py-3 text-sm text-gray-500">{reason}</td>
        <td class="px-4 py-3 text-sm">{expires}</td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        <td class="px-4 py-3 text-sm"><button onclick="if(confirm('确定解除禁言？'))location.href='/admin/users/{uid}/unmute'" class="text-xs text-green-600 hover:underline">解除</button></td>
      </tr>"#,
            username = html_escape(&m.username),
            reason = html_escape(&m.reason),
            expires = expires_text,
            time = m.created_at.chars().take(16).collect::<String>(),
            uid = m.user_id,
        )
    }).collect::<Vec<_>>().join("\n");

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">黑名单管理</h1>
  <!-- Add form -->
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-5 mb-6 fade-in">
    <h3 class="font-semibold mb-4"><i class="fa fa-plus-circle mr-1"></i>添加黑名单</h3>
    <form method="POST" action="/admin/blacklist/add" accept-charset="UTF-8" class="flex gap-3 items-end">
      <div class="w-32"><label class="text-xs text-gray-500">类型</label>
        <select name="type" class="w-full border rounded-lg px-3 py-2 text-sm"><option value="ip">IP 地址</option><option value="user">用户 ID</option></select>
      </div>
      <div class="flex-1"><label class="text-xs text-gray-500">值</label>
        <input name="value" required placeholder="如: 192.168.1.1 或用户ID" class="w-full border rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
      </div>
      <div class="flex-1"><label class="text-xs text-gray-500">原因</label>
        <input name="reason" placeholder="封禁原因" class="w-full border rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
      </div>
      <button type="submit" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800"><i class="fa fa-plus mr-1"></i>添加</button>
    </form>
  </div>
  <!-- IP blacklist -->
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden mb-6 fade-in">
    <div class="px-5 py-4 border-b border-gray-100"><h3 class="font-semibold">IP 黑名单</h3></div>
    <table class="w-full"><thead class="bg-gray-50 text-xs text-gray-500"><tr><th class="px-4 py-3 text-left">IP</th><th class="px-4 py-3 text-left">原因</th><th class="px-4 py-3 text-left w-32">时间</th><th class="px-4 py-3 text-left w-16">操作</th></tr></thead><tbody>{ip_rows}</tbody></table>
    {ip_empty}
  </div>
  <!-- User blacklist -->
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden mb-6 fade-in">
    <div class="px-5 py-4 border-b border-gray-100"><h3 class="font-semibold">用户封禁</h3></div>
    <table class="w-full"><thead class="bg-gray-50 text-xs text-gray-500"><tr><th class="px-4 py-3 text-left">用户ID</th><th class="px-4 py-3 text-left">原因</th><th class="px-4 py-3 text-left w-32">时间</th><th class="px-4 py-3 text-left w-16">操作</th></tr></thead><tbody>{user_rows}</tbody></table>
    {user_empty}
  </div>
  <!-- Muted users -->
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden fade-in">
    <div class="px-5 py-4 border-b border-gray-100"><h3 class="font-semibold">禁言用户</h3></div>
    <table class="w-full"><thead class="bg-gray-50 text-xs text-gray-500"><tr><th class="px-4 py-3 text-left">用户名</th><th class="px-4 py-3 text-left">原因</th><th class="px-4 py-3 text-left">到期</th><th class="px-4 py-3 text-left w-32">时间</th><th class="px-4 py-3 text-left w-16">操作</th></tr></thead><tbody>{muted_rows}</tbody></table>
    {muted_empty}
  </div>"#,
        ip_empty = if ip_rows.is_empty() { r#"<div class="text-center text-gray-400 text-sm py-8">暂无 IP 黑名单</div>"# } else { "" },
        user_empty = if user_rows.is_empty() { r#"<div class="text-center text-gray-400 text-sm py-8">暂无用户封禁</div>"# } else { "" },
        muted_empty = if muted_rows.is_empty() { r#"<div class="text-center text-gray-400 text-sm py-8">暂无禁言用户</div>"# } else { "" },
    );
    admin_layout("黑名单管理", "blacklist", &content)
}

// ------------------------------------------------------------------
// Invite Codes Page
// ------------------------------------------------------------------

pub fn render_admin_invite_codes(codes: &[crate::handlers::admin::InviteCodeRow], _admin_id: i64) -> String {
    let rows = if codes.is_empty() {
        r#"<div class="text-center text-gray-400 text-sm py-8">暂无邀请码</div>"#.to_string()
    } else {
        codes.iter().map(|c| {
            let status_text = if c.used_count >= c.max_uses { "已用完" } else { "可用" };
            let status_cls = if c.used_count >= c.max_uses { "bg-gray-100 text-gray-500" } else { "bg-green-100 text-green-700" };
            format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm font-mono">{code}</td>
        <td class="px-4 py-3 text-sm">{used}/{max}</td>
        <td class="px-4 py-3 text-sm"><span class="px-2 py-0.5 rounded text-xs {status_cls}">{status_text}</span></td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
        <td class="px-4 py-3 text-sm"><button onclick="if(confirm('确定删除？'))postAction('/admin/invite-codes/{id}/delete')" class="text-xs text-red-500 hover:underline">删除</button></td>
      </tr>"#,
                code = c.code,
                used = c.used_count,
                max = c.max_uses,
                status_cls = status_cls,
                status_text = status_text,
                time = c.created_at.chars().take(16).collect::<String>(),
                id = c.id,
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">邀请码管理</h1>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-5 mb-6 fade-in">
    <h3 class="font-semibold mb-4"><i class="fa fa-plus-circle mr-1"></i>生成邀请码</h3>
    <form method="POST" action="/admin/invite-codes/create" accept-charset="UTF-8" class="flex gap-3 items-end">
      <div class="w-32"><label class="text-xs text-gray-500">数量</label>
        <input name="count" type="number" value="5" min="1" max="100" class="w-full border rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
      </div>
      <div class="w-32"><label class="text-xs text-gray-500">每码可用次数</label>
        <input name="max_uses" type="number" value="1" min="1" class="w-full border rounded-lg px-3 py-2 text-sm outline-none focus:border-black">
      </div>
      <button type="submit" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800"><i class="fa fa-plus mr-1"></i>生成</button>
    </form>
  </div>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden fade-in">
    <table class="w-full">
      <thead class="bg-gray-50 text-xs text-gray-500"><tr>
        <th class="px-4 py-3 text-left">邀请码</th><th class="px-4 py-3 text-left">使用量</th><th class="px-4 py-3 text-left">状态</th><th class="px-4 py-3 text-left w-32">创建时间</th><th class="px-4 py-3 text-left w-16">操作</th>
      </tr></thead>
      <tbody>{rows}</tbody>
    </table>
    {empty}
  </div>"#,
        rows = rows,
        empty = if codes.is_empty() { "" } else { "" },
    );
    admin_layout("邀请码管理", "invite", &content)
}

// ------------------------------------------------------------------
// AI Review Page
// ------------------------------------------------------------------

pub fn render_admin_review(settings: &std::collections::HashMap<String, String>) -> String {
    let enabled = settings.get("ai_review_enabled").map(|v| v.as_str()).unwrap_or("0");
    let api_url = settings.get("ai_review_api_url").map(|v| v.as_str()).unwrap_or("");
    let prompt = settings.get("ai_review_prompt").map(|v| v.as_str()).unwrap_or("");

    let enabled_badge = if enabled == "1" {
        r#"<span class="bg-green-100 text-green-700 text-xs px-2 py-0.5 rounded">已启用</span>"#
    } else {
        r#"<span class="bg-gray-100 text-gray-600 text-xs px-2 py-0.5 rounded">未启用</span>"#
    };

    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">安全审查</h1>
  <div class="mb-4">AI 审查状态: {enabled_badge}</div>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 mb-6 fade-in">
    <h3 class="font-semibold mb-4"><i class="fa fa-search mr-1"></i>内容审查</h3>
    <textarea id="reviewContent" rows="8" class="w-full border rounded-lg px-4 py-3 text-sm outline-none focus:border-black resize-y mb-4" placeholder="输入或粘贴需要审查的内容..."></textarea>
    <button onclick="doAiReview()" id="reviewBtn" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800">
      <i class="fa fa-shield mr-1"></i>开始审查
    </button>
    <div id="reviewResult" class="mt-4 hidden">
      <div class="border rounded-lg p-4">
        <h4 class="font-semibold mb-2">审查结果</h4>
        <div id="reviewResultContent"></div>
      </div>
    </div>
  </div>
  <!-- Review config preview -->
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 fade-in">
    <h3 class="font-semibold mb-4">当前审查配置</h3>
    <div class="text-sm space-y-2">
      <div><span class="text-gray-500">API URL:</span> <span class="font-mono text-xs">{api_url}</span></div>
      <div><span class="text-gray-500">系统提示词:</span> <pre class="bg-gray-50 p-3 rounded text-xs mt-1 whitespace-pre-wrap">{prompt_preview}</pre></div>
    </div>
    <a href="/admin/settings/ai" class="inline-block mt-4 bg-gray-100 px-4 py-2 rounded-lg text-sm hover:bg-gray-200"><i class="fa fa-cog mr-1"></i>修改设置</a>
  </div>
  <script>
  function doAiReview() {{
    var content = document.getElementById('reviewContent').value;
    if(!content) {{ showToast('请输入内容'); return; }}
    var btn = document.getElementById('reviewBtn');
    btn.disabled = true; btn.textContent = '审查中...';
    fetch('/admin/review/check', {{
      method:'POST', headers:{{'Content-Type':'application/x-www-form-urlencoded'}},
      body:'content='+encodeURIComponent(content)
    }}).then(function(r){{ return r.json(); }})
    .then(function(data){{
      btn.disabled = false; btn.innerHTML = '<i class="fa fa-shield mr-1"></i>开始审查';
      var el = document.getElementById('reviewResult');
      el.classList.remove('hidden');
      var levelColor = data.level==='safe'?'green':data.level==='warning'?'yellow':'red';
      document.getElementById('reviewResultContent').innerHTML =
        '<div class="flex items-center gap-2 mb-2"><span class="text-lg font-bold text-'+levelColor+'-600">'+(data.safe?'安全':'可疑')+'</span><span class="bg-'+levelColor+'-100 text-'+levelColor+'-700 px-2 py-0.5 rounded text-xs">'+data.level+'</span></div>'
        + '<p class="text-sm text-gray-600">'+(data.reason||'无说明')+'</p>';
    }}).catch(function(e){{
      btn.disabled = false; btn.innerHTML = '<i class="fa fa-shield mr-1"></i>开始审查';
      showToast('审查失败: '+e.message);
    }});
  }}
  </script>"#,
        enabled_badge = enabled_badge,
        api_url = if api_url.is_empty() { "未配置".to_string() } else { html_escape(api_url) },
        prompt_preview = html_escape(prompt),
    );
    admin_layout("安全审查", "review", &content)
}

// ------------------------------------------------------------------
// Settings Pages
// ------------------------------------------------------------------

fn settings_field(label: &str, name: &str, value: &str, input_type: &str) -> String {
    if input_type == "textarea" {
        format!(r#"<div class="mb-5"><label class="block text-sm font-medium mb-2">{label}</label>
      <textarea name="{name}" rows="3" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black resize-y">{value}</textarea></div>"#,
            label = label, name = name, value = html_escape(value))
    } else if input_type == "select" {
        let checked_yes = if value == "1" { "selected" } else { "" };
        let checked_no = if value != "1" { "selected" } else { "" };
        format!(r#"<div class="mb-5"><label class="block text-sm font-medium mb-2">{label}</label>
      <select name="{name}" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black">
        <option value="1" {yes}>开启</option><option value="0" {no}>关闭</option>
      </select></div>"#,
            label = label, name = name, yes = checked_yes, no = checked_no)
    } else {
        format!(r#"<div class="mb-5"><label class="block text-sm font-medium mb-2">{label}</label>
      <input type="{input_type}" name="{name}" value="{value}" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black"></div>"#,
            label = label, name = name, value = html_escape(value), input_type = input_type)
    }
}

fn settings_page_wrapper(title: &str, active_sub: &str, fields_html: &str) -> String {
    let content = format!(r#"
  <h1 class="text-2xl font-bold mb-6">系统设置</h1>
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 fade-in">
    <h3 class="font-semibold mb-6">{title}</h3>
    <form method="POST" accept-charset="UTF-8">
      {fields_html}
      <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800">
        <i class="fa fa-save mr-1"></i>保存设置
      </button>
    </form>
  </div>"#);
    admin_layout(&format!("系统设置 - {}", title), &format!("settings_{}", active_sub), &content)
}

pub fn render_settings_site(settings: &std::collections::HashMap<String, String>) -> String {
    let fields = vec![
        settings_field("站点名称", "site_name", settings.get("site_name").map(|v| v.as_str()).unwrap_or(""), "text"),
        settings_field("站点描述", "site_description", settings.get("site_description").map(|v| v.as_str()).unwrap_or(""), "textarea"),
        settings_field("站点关键词", "site_keywords", settings.get("site_keywords").map(|v| v.as_str()).unwrap_or(""), "text"),
        settings_field("页脚文字", "site_footer_text", settings.get("site_footer_text").map(|v| v.as_str()).unwrap_or(""), "text"),
    ];
    settings_page_wrapper("站点信息", "site", &fields.join("\n"))
}

pub fn render_settings_register(settings: &std::collections::HashMap<String, String>) -> String {
    let fields = vec![
        settings_field("允许注册", "allow_register", settings.get("allow_register").map(|v| v.as_str()).unwrap_or("1"), "select"),
        settings_field("需要邀请码", "invite_required", settings.get("invite_required").map(|v| v.as_str()).unwrap_or("0"), "select"),
    ];
    settings_page_wrapper("注册设置", "register", &fields.join("\n"))
}

pub fn render_settings_credits(settings: &std::collections::HashMap<String, String>) -> String {
    let fields = vec![
        settings_field("签到积分", "credits_checkin", settings.get("credits_checkin").map(|v| v.as_str()).unwrap_or("5"), "number"),
        settings_field("发帖积分", "credits_thread", settings.get("credits_thread").map(|v| v.as_str()).unwrap_or("3"), "number"),
        settings_field("回复积分", "credits_reply", settings.get("credits_reply").map(|v| v.as_str()).unwrap_or("2"), "number"),
        settings_field("精华积分", "credits_essence", settings.get("credits_essence").map(|v| v.as_str()).unwrap_or("20"), "number"),
    ];
    settings_page_wrapper("积分设置", "credits", &fields.join("\n"))
}

pub fn render_settings_upload(settings: &std::collections::HashMap<String, String>) -> String {
    let fields = vec![
        settings_field("头像最大大小（字节）", "max_avatar_size", settings.get("max_avatar_size").map(|v| v.as_str()).unwrap_or("524288"), "number"),
    ];
    settings_page_wrapper("上传设置", "upload", &fields.join("\n"))
}

pub fn render_settings_ai(settings: &std::collections::HashMap<String, String>) -> String {
    let fields = vec![
        settings_field("启用 AI 审查", "ai_review_enabled", settings.get("ai_review_enabled").map(|v| v.as_str()).unwrap_or("0"), "select"),
        settings_field("API URL", "ai_review_api_url", settings.get("ai_review_api_url").map(|v| v.as_str()).unwrap_or(""), "text"),
        settings_field("API Key", "ai_review_api_key", settings.get("ai_review_api_key").map(|v| v.as_str()).unwrap_or(""), "text"),
        settings_field("模型名称", "ai_review_model", settings.get("ai_review_model").map(|v| v.as_str()).unwrap_or("gpt-4o-mini"), "text"),
        settings_field("审查提示词", "ai_review_prompt", settings.get("ai_review_prompt").map(|v| v.as_str()).unwrap_or(""), "textarea"),
    ];
    settings_page_wrapper("AI 审查设置", "ai", &fields.join("\n"))
}

pub fn render_settings_email(settings: &std::collections::HashMap<String, String>) -> String {
    let provider = settings.get("email_provider").map(|v| v.as_str()).unwrap_or("smtp");
    let encryption = settings.get("email_smtp_encryption").map(|v| v.as_str()).unwrap_or("tls");

    let provider_select = format!(r#"<div class="mb-5"><label class="block text-sm font-medium mb-2">发送方式</label>
      <select name="email_provider" id="email_provider" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black">
        <option value="smtp" {sel_smtp}>SMTP</option>
        <option value="sendflare" {sel_sf}>SendFlare API</option>
      </select></div>"#,
        sel_smtp = if provider == "smtp" { "selected" } else { "" },
        sel_sf = if provider == "sendflare" { "selected" } else { "" },
    );

    let sendflare_fields = format!(r#"<div id="sendflare-fields" class="border border-gray-200 rounded-lg p-4 mb-5 {sf_display}">
      <h4 class="text-sm font-semibold mb-3 text-gray-700"><i class="fa fa-cloud mr-1"></i>SendFlare API 配置</h4>
      {api_url}
      {api_key}
    </div>"#,
        sf_display = if provider == "sendflare" { "" } else { "hidden" },
        api_url = settings_field("API URL", "email_sendflare_api_url", settings.get("email_sendflare_api_url").map(|v| v.as_str()).unwrap_or("https://api.sendflare.com"), "text"),
        api_key = settings_field("API Key", "email_sendflare_api_key", settings.get("email_sendflare_api_key").map(|v| v.as_str()).unwrap_or(""), "text"),
    );

    let smtp_fields = format!(r#"<div id="smtp-fields" class="border border-gray-200 rounded-lg p-4 mb-5 {smtp_display}">
      <h4 class="text-sm font-semibold mb-3 text-gray-700"><i class="fa fa-server mr-1"></i>SMTP 配置</h4>
      {host}
      {port}
      <div class="mb-5"><label class="block text-sm font-medium mb-2">加密方式</label>
        <select name="email_smtp_encryption" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black">
          <option value="tls" {sel_tls}>SSL/TLS (端口 465)</option>
          <option value="starttls" {sel_start}>STARTTLS (端口 587)</option>
        </select></div>
      {username}
      {password}
    </div>"#,
        smtp_display = if provider == "smtp" { "" } else { "hidden" },
        host = settings_field("SMTP 服务器", "email_smtp_host", settings.get("email_smtp_host").map(|v| v.as_str()).unwrap_or(""), "text"),
        port = settings_field("SMTP 端口", "email_smtp_port", settings.get("email_smtp_port").map(|v| v.as_str()).unwrap_or("465"), "number"),
        sel_tls = if encryption == "tls" { "selected" } else { "" },
        sel_start = if encryption == "starttls" { "selected" } else { "" },
        username = settings_field("SMTP 用户名", "email_smtp_username", settings.get("email_smtp_username").map(|v| v.as_str()).unwrap_or(""), "text"),
        password = settings_field("SMTP 密码", "email_smtp_password", settings.get("email_smtp_password").map(|v| v.as_str()).unwrap_or(""), "text"),
    );

    let common_fields = vec![
        settings_field("启用邮件服务", "email_enabled", settings.get("email_enabled").map(|v| v.as_str()).unwrap_or("0"), "select"),
        settings_field("发件人名称", "email_from_name", settings.get("email_from_name").map(|v| v.as_str()).unwrap_or(""), "text"),
        settings_field("发件人地址", "email_from_address", settings.get("email_from_address").map(|v| v.as_str()).unwrap_or(""), "text"),
    ];

    let verify_fields = format!(r#"<div class="border border-gray-200 rounded-lg p-4 mb-5">
      <h4 class="text-sm font-semibold mb-3 text-gray-700"><i class="fa fa-shield mr-1"></i>邮箱验证设置</h4>
      {verify_enabled}
      {expire_hours}
      {site_url}
    </div>"#,
        verify_enabled = settings_field("注册邮箱验证", "email_verification_enabled", settings.get("email_verification_enabled").map(|v| v.as_str()).unwrap_or("0"), "select"),
        expire_hours = settings_field("验证链接有效期（小时）", "email_verify_expire_hours", settings.get("email_verify_expire_hours").map(|v| v.as_str()).unwrap_or("24"), "number"),
        site_url = settings_field("站点 URL（用于生成验证链接）", "site_url", settings.get("site_url").map(|v| v.as_str()).unwrap_or("http://localhost:3000"), "text"),
    );

    let test_section = r#"<div class="border border-gray-200 rounded-lg p-4 mb-5 bg-gray-50">
      <h4 class="text-sm font-semibold mb-3 text-gray-700"><i class="fa fa-paper-plane mr-1"></i>发送测试邮件</h4>
      <div class="flex gap-2 items-end">
        <div class="flex-1"><label class="block text-xs text-gray-500 mb-1">收件人地址</label>
          <input type="email" id="test_email_to" placeholder="test@example.com" class="w-full border border-gray-200 rounded-lg px-3 py-2 text-sm outline-none focus:border-black"></div>
        <button type="button" onclick="sendTestEmail()" id="test_btn" class="bg-blue-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-blue-700 flex-shrink-0">
          <i class="fa fa-paper-plane mr-1"></i>发送测试
        </button>
      </div>
      <div id="test_result" class="mt-2 text-sm hidden"></div>
    </div>"#;

    let script = r#"<script>
function sendTestEmail() {
  const to = document.getElementById('test_email_to').value.trim();
  if (!to) { alert('请输入收件人地址'); return; }
  const btn = document.getElementById('test_btn');
  const result = document.getElementById('test_result');
  btn.disabled = true;
  btn.innerHTML = '<i class="fa fa-spinner fa-spin mr-1"></i>发送中...';
  result.classList.add('hidden');
  fetch('/admin/settings/email/test', {
    method: 'POST',
    headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({to: to})
  })
  .then(r => r.json())
  .then(data => {
    btn.disabled = false;
    btn.innerHTML = '<i class="fa fa-paper-plane mr-1"></i>发送测试';
    result.classList.remove('hidden');
    if (data.ok) {
      result.className = 'mt-2 text-sm text-green-600';
      result.innerHTML = '<i class="fa fa-check-circle mr-1"></i>' + data.message;
    } else {
      result.className = 'mt-2 text-sm text-red-600';
      result.innerHTML = '<i class="fa fa-exclamation-circle mr-1"></i>' + data.message;
    }
  })
  .catch(err => {
    btn.disabled = false;
    btn.innerHTML = '<i class="fa fa-paper-plane mr-1"></i>发送测试';
    result.classList.remove('hidden');
    result.className = 'mt-2 text-sm text-red-600';
    result.innerHTML = '<i class="fa fa-exclamation-circle mr-1"></i>请求失败: ' + err;
  });
}
document.getElementById('email_provider').addEventListener('change', function() {
  const v = this.value;
  document.getElementById('sendflare-fields').classList.toggle('hidden', v !== 'sendflare');
  document.getElementById('smtp-fields').classList.toggle('hidden', v !== 'smtp');
});
</script>"#;

    let all_fields = format!(
        "{}{}{}{}{}{}{}",
        common_fields.join("\n"),
        provider_select,
        sendflare_fields,
        smtp_fields,
        verify_fields,
        test_section,
        script,
    );

    settings_page_wrapper("邮件设置", "email", &all_fields)
}

// =====================================================================
// Helpers
// =====================================================================

fn pagination_html(current: i64, total: i64, base_url: &str) -> String {
    if total <= 1 { return String::new(); }
    let mut html = r#"<div class="flex justify-center mt-10"><div class="flex gap-1">"#.to_string();
    if current > 1 {
        html.push_str(&format!(r#"<a href="{}?page={}" class="w-9 h-9 flex items-center justify-center rounded hover:bg-gray-100 text-gray-600"><i class="fa fa-angle-left"></i></a>"#, base_url, current - 1));
    }
    let start = (current - 3).max(1);
    let end = (current + 3).min(total);
    for i in start..=end {
        if i == current {
            html.push_str(&format!(r#"<span class="w-9 h-9 flex items-center justify-center rounded bg-black text-white">{}</span>"#, i));
        } else {
            html.push_str(&format!(r#"<a href="{}?page={}" class="w-9 h-9 flex items-center justify-center rounded hover:bg-gray-100">{}</a>"#, base_url, i, i));
        }
    }
    if current < total {
        html.push_str(&format!(r#"<a href="{}?page={}" class="w-9 h-9 flex items-center justify-center rounded hover:bg-gray-100 text-gray-600"><i class="fa fa-angle-right"></i></a>"#, base_url, current + 1));
    }
    html.push_str("</div></div>");
    html
}

pub fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

pub fn urlencoding(s: &str) -> String {
    s.chars().map(|c| {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            c.to_string()
        } else {
            format!("%{:02X}", c as u32)
        }
    }).collect()
}

fn render_markdown(text: &str) -> String {
    use pulldown_cmark::{Options, Parser, html};
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    let parser = Parser::new_ext(text, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);
    html_output
}

fn render_content(content: &str) -> String {
    let mut result = String::new();
    let mut remaining = content;
    loop {
        if let Some(start) = remaining.find("[quote=") {
            // Render Markdown for text before the quote
            let before = &remaining[..start];
            if !before.trim().is_empty() {
                result.push_str(&render_markdown(before));
            }
            let after_start = &remaining[start + 7..];
            if let Some(end_name) = after_start.find(']') {
                let name = &after_start[..end_name];
                let after_bracket = &after_start[end_name + 1..];
                if let Some(end_quote) = after_bracket.find("[/quote]") {
                    let quoted = &after_bracket[..end_quote];
                    result.push_str(&format!(
                        r#"<div class="border-l-2 border-gray-300 bg-gray-50 rounded px-3 py-2 mb-2"><div class="text-xs text-gray-500 mb-1"><i class="fa fa-reply"></i> {}</div><div class="text-xs text-gray-600 whitespace-pre-wrap">{}</div></div>"#,
                        html_escape(name),
                        html_escape(quoted),
                    ));
                    remaining = &after_bracket[end_quote + 8..];
                } else {
                    result.push_str(&render_markdown(&remaining[start..]));
                    break;
                }
            } else {
                result.push_str(&render_markdown(&remaining[start..]));
                break;
            }
        } else {
            if !remaining.trim().is_empty() {
                result.push_str(&render_markdown(remaining));
            }
            break;
        }
    }
    result
}

fn truncate_chars(s: &str, max_chars: usize) -> &str {
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

// =====================================================================
// User Profile (view other user)
// =====================================================================

pub fn render_user_profile(
    user: &User,
    recent_threads: &[ThreadList],
    recent_posts: &[Post],
    current_user: Option<&User>,
) -> String {
    let join_date = user.created_at.chars().take(10).collect::<String>();

    let threads_html = if recent_threads.is_empty() {
        r#"<div class="px-5 py-8 text-center text-gray-400 text-sm">暂无发帖</div>"#.to_string()
    } else {
        recent_threads.iter().map(|t| thread_row_html(t)).collect::<Vec<_>>().join("\n")
    };

    let posts_html = if recent_posts.is_empty() {
        r#"<div class="px-5 py-8 text-center text-gray-400 text-sm">暂无回复</div>"#.to_string()
    } else {
        recent_posts.iter().map(|p| {
            let content_preview = if p.content.chars().count() > 80 {
                format!("{}...", html_escape(truncate_chars(&p.content, 80)))
            } else {
                html_escape(&p.content)
            };
            format!(
                r#"<div class="item-hover px-5 py-4 border-b border-gray-100 cursor-pointer" onclick="location.href='/thread/{thread_id}?page={page}#floor-{floor}'">
          <div class="flex items-center justify-between">
            <div class="flex-1 min-w-0">
              <p class="text-sm truncate">{content}</p>
              <div class="flex items-center gap-3 mt-1 text-xs text-gray-500">
                <span>#{floor} 楼</span>
                <span>{time}</span>
              </div>
            </div>
            <i class="fa fa-angle-right text-gray-300 ml-3"></i>
          </div>
        </div>"#,
                thread_id = p.thread_id,
                floor = p.floor,
                page = (p.floor - 1) / 20 + 1,
                content = content_preview,
                time = p.created_at.chars().take(10).collect::<String>(),
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <span class="text-black">用户资料</span></div>

    <!-- Profile header card -->
    <div class="bg-white border border-gray-200 rounded-lg p-6 mb-6 fade-in">
      <div class="flex items-center gap-5">
        {user_profile_avatar}
        <div>
          <h2 class="text-xl font-semibold">{username}</h2>
          <div class="flex items-center gap-3 mt-1 text-xs text-gray-500">
            <span class="bg-gray-100 px-2 py-0.5 rounded">{group}</span>
            <span><i class="fa fa-calendar"></i> {join_date} 加入</span>
          </div>
        </div>
      </div>
      <div class="w-full h-px bg-gray-200 my-4"></div>
      <div class="grid grid-cols-3 gap-4 text-center">
        <div><p class="text-lg font-semibold">{threads}</p><p class="text-xs text-gray-500">主题</p></div>
        <div><p class="text-lg font-semibold">{posts}</p><p class="text-xs text-gray-500">帖子</p></div>
        <div><p class="text-lg font-semibold">{credits}</p><p class="text-xs text-gray-500">积分</p></div>
      </div>
      {signature}
    </div>

    <!-- Recent threads -->
    <div class="mb-6">
      <h3 class="font-semibold mb-3">{username} 的主题</h3>
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden">
        {threads_html}
      </div>
    </div>

    <!-- Recent posts -->
    <div>
      <h3 class="font-semibold mb-3">最近回复</h3>
      <div class="bg-white border border-gray-200 rounded-lg overflow-hidden">
        {posts_html}
      </div>
    </div>"#,
        user_profile_avatar = avatar_html(&user.avatar, user.id, &user.username, "w-16 h-16 text-2xl"),
        username = html_escape(&user.username),
        group = user.group_name(),
        join_date = join_date,
        threads = user.thread_count,
        posts = user.post_count,
        credits = user.credits,
        signature = if user.signature.is_empty() {
            String::new()
        } else {
            format!(r#"<div class="mt-4 p-3 bg-gray-50 rounded-lg text-sm text-gray-600"><i class="fa fa-pencil mr-1"></i> {}</div>"#, html_escape(&user.signature))
        },
        threads_html = threads_html,
        posts_html = posts_html,
    );

    page_with_sidebar(&user.username, &main, current_user, "", "home")
}

// =====================================================================
// Profile Edit Page
// =====================================================================

pub fn render_profile_edit(user: &User, email_unverified: bool) -> String {
    let current_avatar = avatar_html(&user.avatar, user.id, &user.username, "w-20 h-20 text-3xl");
    let has_avatar = !user.avatar.is_empty();
    let delete_btn = if has_avatar {
        r#"<button type="button" onclick="if(confirm('确定删除头像？'))postAction('/profile/avatar/delete')" class="bg-gray-100 text-red-500 px-4 py-2 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors"><i class="fa fa-trash-o mr-1"></i>删除头像</button>"#
    } else {
        ""
    };

    let email_status = if email_unverified {
        r#"<p class="text-xs text-orange-500 mt-1"><i class="fa fa-exclamation-circle mr-1"></i>邮箱未验证，请查收验证邮件。<a href="/auth/resend-verify" class="text-black font-medium hover:underline">重新发送</a></p>"#
    } else {
        ""
    };

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/profile" class="hover:text-black">个人中心</a> <i class="fa fa-angle-right"></i> <span class="text-black">编辑资料</span></div>

    <!-- Avatar upload -->
    <div class="bg-white border border-gray-200 rounded-lg p-6 mb-6 fade-in">
      <h2 class="text-xl font-semibold mb-4">头像</h2>
      <div class="flex items-center gap-6">
        <div class="flex-shrink-0">{current_avatar}</div>
        <div>
          <form id="avatarForm" onsubmit="submitAvatar(event)" class="flex items-center gap-3">
            <input type="file" name="avatar" id="avatarInput" accept="image/jpeg,image/png,image/gif,image/webp" required
              class="text-sm">
            <button type="submit" class="bg-black text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
              <i class="fa fa-upload mr-1"></i>上传
            </button>
          </form>
          <p class="text-xs text-gray-400 mt-2">支持 JPG/PNG/GIF/WebP，最大 512KB</p>
          <div class="mt-2">{delete_btn}</div>
        </div>
      </div>
    </div>

    <!-- Edit profile form -->
    <div class="bg-white border border-gray-200 rounded-lg p-6 mb-6 fade-in">
      <h2 class="text-xl font-semibold mb-6">编辑资料</h2>
      <form onsubmit="submitProfileEdit(event)" accept-charset="UTF-8">
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">用户名</label>
          <input type="text" value="{username}" disabled
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm bg-gray-50 text-gray-500">
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">邮箱</label>
          <input type="email" name="email" id="editEmail" value="{email}" required
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
          {email_status}
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">个性签名</label>
          <textarea name="signature" id="editSignature" rows="3" placeholder="写点什么介绍自己..."
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-y">{signature}</textarea>
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">自定义头衔</label>
          <input type="text" name="custom_title" id="editCustomTitle" value="{custom_title}" placeholder="留空则显示积分头衔" maxlength="20"
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
          <p class="text-xs text-gray-400 mt-1">当前积分头衔：{rank_title}</p>
        </div>
        <div class="grid grid-cols-2 gap-4 mb-5">
          <div>
            <label class="block text-sm font-medium mb-2">称号</label>
            <input type="text" name="epithet" id="editEpithet" value="{epithet}" placeholder="如：大佬、萌新" maxlength="10"
              class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
          </div>
          <div>
            <label class="block text-sm font-medium mb-2">称号颜色</label>
            <div class="flex items-center gap-2">
              <input type="color" name="epithet_color" id="editEpithetColor" value="{epithet_color}"
                class="w-10 h-10 rounded border border-gray-200 cursor-pointer p-1">
              <span class="text-xs text-gray-400">选择称号背景色</span>
            </div>
          </div>
        </div>
        <div class="flex gap-3">
          <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
            <i class="fa fa-save mr-1"></i>保存
          </button>
          <a href="/profile" class="bg-gray-100 text-black px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors">取消</a>
        </div>
      </form>
    </div>

    <!-- Change password form -->
    <div class="bg-white border border-gray-200 rounded-lg p-6 fade-in">
      <h2 class="text-xl font-semibold mb-6">修改密码</h2>
      <form onsubmit="submitChangePassword(event)" accept-charset="UTF-8">
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">旧密码</label>
          <input type="password" name="old_password" id="oldPassword" required
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">新密码</label>
          <input type="password" name="new_password" id="newPassword" required minlength="6"
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
        </div>
        <div class="mb-6">
          <label class="block text-sm font-medium mb-2">确认新密码</label>
          <input type="password" name="confirm_password" id="confirmPassword" required minlength="6"
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
        </div>
        <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
          <i class="fa fa-lock mr-1"></i>修改密码
        </button>
      </form>
    </div>"#,
        username = html_escape(&user.username),
        email = html_escape(&user.email),
        email_status = email_status,
        signature = html_escape(&user.signature),
        custom_title = html_escape(&user.custom_title),
        rank_title = html_escape(user.rank_title()),
        epithet = html_escape(&user.epithet),
        epithet_color = if user.epithet_color.is_empty() { "#8B5CF6".to_string() } else { html_escape(&user.epithet_color) },
    );

    page_with_sidebar("编辑资料", &main, Some(user), "", "home")
}

// =====================================================================
// Verify Email Code Page
// =====================================================================

pub fn render_verify_email_code(email: &str) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>验证邮箱 | {site_name}</title>
  <script src="/static/css/tailwind.js"></script>
  <link rel="stylesheet" href="/static/css/font-awesome.min.css">
  <style>
    body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, 'Helvetica Neue', Arial, sans-serif; }}
    .code-input {{
      letter-spacing: 12px;
      font-size: 24px;
      font-weight: bold;
      text-align: center;
      font-family: monospace;
    }}
  </style>
</head>
<body class="bg-gray-50 min-h-screen">
  <div class="min-h-screen flex items-center justify-center px-4">
    <div class="bg-white border border-gray-200 rounded-xl p-8 w-full max-w-md shadow-sm">
      <div class="text-center mb-8">
        <div class="w-16 h-16 bg-black rounded-full flex items-center justify-center mx-auto mb-4">
          <i class="fa fa-envelope text-white text-2xl"></i>
        </div>
        <h1 class="text-xl font-semibold text-gray-900">验证新邮箱</h1>
        <p class="text-sm text-gray-500 mt-2">
          验证码已发送至 <span class="font-medium text-black">{email}</span>
        </p>
      </div>
      <form method="POST" action="/profile/verify-email" id="verifyForm">
        <div class="mb-6">
          <label class="block text-sm font-medium text-gray-700 mb-2">输入6位验证码</label>
          <input type="text" name="code" id="codeInput" maxlength="6" pattern="[0-9]{{6}}"
            required autocomplete="off" placeholder="000000"
            class="code-input w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
        </div>
        <button type="submit" id="submitBtn" class="w-full bg-black text-white py-3 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
          验证
        </button>
      </form>
      <div class="mt-4 text-center">
        <a href="/profile/edit" class="text-xs text-gray-400 hover:text-black">返回编辑资料</a>
      </div>
    </div>
  </div>
  <script>
    var input = document.getElementById('codeInput');
    input.addEventListener('input', function() {{
      this.value = this.value.replace(/[^0-9]/g, '');
      if (this.value.length === 6) {{
        document.getElementById('verifyForm').submit();
      }}
    }});
    input.focus();
  </script>
</body>
</html>"##,
        site_name = "开发者社区",
        email = html_escape(email),
    )
}
// Edit Thread Page
// =====================================================================

pub fn render_edit_thread(thread: &Thread, content: &str) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>编辑帖子 | {site_name}</title>
  <script src="/static/css/tailwind.js"></script>
  <link href="/static/css/font-awesome.min.css" rel="stylesheet">
</head>
<body class="bg-white text-black min-h-screen">
<div class="container mx-auto px-4 py-10 max-w-3xl">
  <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/thread/{thread_id}" class="hover:text-black">{title}</a> <i class="fa fa-angle-right"></i> <span class="text-black">编辑</span></div>
  <div class="bg-white border border-gray-200 rounded-lg p-6">
    <h2 class="text-xl font-semibold mb-6">编辑帖子</h2>
    <form method="POST" action="/thread/{thread_id}/edit" accept-charset="UTF-8">
      <div class="mb-5">
        <label class="block text-sm font-medium mb-2">标题</label>
        <input type="text" name="title" value="{title}" required
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
      </div>
      <div class="mb-5">
        <label class="block text-sm font-medium mb-2">内容</label>
        <textarea name="content" rows="12" required
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-y">{content}</textarea>
      </div>
      <div class="flex gap-3">
        <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
          <i class="fa fa-save mr-1"></i>保存
        </button>
        <a href="/thread/{thread_id}" class="bg-gray-100 text-black px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors">取消</a>
      </div>
    </form>
  </div>
</div>
<div id="toast" class="fixed top-20 right-4 z-[999] hidden">
  <div class="bg-black text-white px-5 py-3 rounded-lg shadow-lg text-sm" id="toastMsg"></div>
</div>
<script src="/static/js/app.js"></script>
</body>
</html>"##,
        thread_id = thread.id,
        title = html_escape(&thread.title),
        content = html_escape(content),
        site_name = crate::site_config::site_name(),
    )
}

// =====================================================================
// Edit Post Page
// =====================================================================

pub fn render_edit_post(post: &Post, thread_title: &str) -> String {
    format!(r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>编辑回复 | {site_name}</title>
  <script src="/static/css/tailwind.js"></script>
  <link href="/static/css/font-awesome.min.css" rel="stylesheet">
</head>
<body class="bg-white text-black min-h-screen">
<div class="container mx-auto px-4 py-10 max-w-3xl">
  <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/thread/{thread_id}" class="hover:text-black">{thread_title}</a> <i class="fa fa-angle-right"></i> <span class="text-black">编辑回复</span></div>
  <div class="bg-white border border-gray-200 rounded-lg p-6">
    <h2 class="text-xl font-semibold mb-6">编辑回复</h2>
    <form method="POST" action="/post/{post_id}/edit" accept-charset="UTF-8">
      <div class="mb-5">
        <label class="block text-sm font-medium mb-2">内容</label>
        <textarea name="content" rows="10" required
          class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-y">{content}</textarea>
      </div>
      <div class="flex gap-3">
        <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
          <i class="fa fa-save mr-1"></i>保存
        </button>
        <a href="/thread/{thread_id}" class="bg-gray-100 text-black px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors">取消</a>
      </div>
    </form>
  </div>
</div>
<div id="toast" class="fixed top-20 right-4 z-[999] hidden">
  <div class="bg-black text-white px-5 py-3 rounded-lg shadow-lg text-sm" id="toastMsg"></div>
</div>
<script src="/static/js/app.js"></script>
</body>
</html>"##,
        post_id = post.id,
        thread_id = post.thread_id,
        thread_title = html_escape(thread_title),
        content = html_escape(&post.content),
        site_name = crate::site_config::site_name(),
    )
}

// =====================================================================
// Message: Inbox
// =====================================================================

pub fn render_inbox(conversations: &[Message], user: &User, unread_count: i64) -> String {
    let conversations_html = if conversations.is_empty() {
        r#"<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无消息</div>"#.to_string()
    } else {
        conversations.iter().map(|m| {
            let is_sender = m.sender_id == user.id;
            let partner_id = if is_sender { m.receiver_id } else { m.sender_id };
            let partner_name = if is_sender {
                m.receiver_name.as_deref().unwrap_or("未知")
            } else {
                m.sender_name.as_deref().unwrap_or("未知")
            };
            let partner_avatar = if is_sender { "" } else { m.sender_avatar.as_deref().unwrap_or("") };

            let unread_badge = if !is_sender && m.is_read == 0 {
                r#"<span class="bg-red-500 text-white text-xs rounded-full w-5 h-5 flex items-center justify-center flex-shrink-0">!</span>"#
            } else {
                ""
            };

            let content_preview = if m.content.chars().count() > 50 {
                format!("{}...", html_escape(truncate_chars(&m.content, 50)))
            } else {
                html_escape(&m.content)
            };

            let partner_avatar_html = avatar_html(partner_avatar, partner_id, partner_name, "w-10 h-10 text-sm");

            format!(
                r#"<a href="/messages/{partner_id}" class="item-hover px-5 py-4 border-b border-gray-100 flex items-center gap-4 hover:bg-gray-50 transition-colors">
          <div class="flex-shrink-0">{partner_avatar_html}</div>
          <div class="flex-1 min-w-0">
            <div class="flex items-center justify-between">
              <span class="font-medium text-sm">{partner_name}</span>
              <span class="text-xs text-gray-400">{time}</span>
            </div>
            <p class="text-xs text-gray-500 mt-1 truncate">{content_preview}</p>
          </div>
          {unread_badge}
        </a>"#,
                partner_id = partner_id,
                partner_avatar_html = partner_avatar_html,
                partner_name = html_escape(partner_name),
                content_preview = content_preview,
                time = m.created_at.chars().take(16).collect::<String>(),
                unread_badge = unread_badge,
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let unread_title = if unread_count > 0 {
        format!("消息 ({})", unread_count)
    } else {
        "消息".to_string()
    };

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <span class="text-black">{unread_title}</span></div>

    <div class="flex justify-between items-center mb-6">
      <h2 class="text-xl font-semibold">消息</h2>
      <a href="/messages/compose" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors"><i class="fa fa-pencil mr-1"></i>写新消息</a>
    </div>

    <div class="bg-white border border-gray-200 rounded-lg overflow-hidden fade-in">
      {conversations_html}
    </div>"#,
        unread_title = unread_title,
        conversations_html = conversations_html,
    );

    page_with_sidebar("消息", &main, Some(user), "", "home")
}

// =====================================================================
// Message: Conversation (chat style)
// =====================================================================

pub fn render_conversation(
    messages: &[Message],
    user: &User,
    partner_id: i64,
    partner_name: &str,
    partner_avatar: &str,
) -> String {
    let partner_avatar_html = avatar_html(partner_avatar, partner_id, partner_name, "w-10 h-10 text-sm");

    let messages_html = if messages.is_empty() {
        r#"<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无消息，发送第一条吧</div>"#.to_string()
    } else {
        messages.iter().map(|m| {
            let is_self = m.sender_id == user.id;
            let time = m.created_at.chars().take(16).collect::<String>();

            if is_self {
                format!(
                    r#"<div class="flex justify-end gap-2 mb-4">
            <div class="max-w-xs lg:max-w-md">
              <div class="bg-black text-white rounded-2xl rounded-br-sm px-4 py-2.5 text-sm">{content}</div>
              <div class="text-xs text-gray-400 mt-1 text-right">{time}</div>
            </div>
          </div>"#,
                    content = html_escape(&m.content),
                    time = time,
                )
            } else {
                format!(
                    r#"<div class="flex gap-2 mb-4">
            <div class="flex-shrink-0 mt-1">{partner_avatar_html}</div>
            <div class="max-w-xs lg:max-w-md">
              <div class="bg-gray-100 rounded-2xl rounded-bl-sm px-4 py-2.5 text-sm">{content}</div>
              <div class="text-xs text-gray-400 mt-1">{time}</div>
            </div>
          </div>"#,
                    partner_avatar_html = partner_avatar_html,
                    content = html_escape(&m.content),
                    time = time,
                )
            }
        }).collect::<Vec<_>>().join("\n")
    };

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/messages" class="hover:text-black">消息</a> <i class="fa fa-angle-right"></i> <span class="text-black">{partner_name}</span></div>

    <div class="flex justify-between items-center mb-6">
      <div class="flex items-center gap-3">
        <a href="/messages" class="text-gray-400 hover:text-black"><i class="fa fa-angle-left"></i></a>
        <a href="/user/{partner_id}" class="flex items-center gap-2">
          {partner_avatar_html}
          <h2 class="text-xl font-semibold">{partner_name}</h2>
        </a>
      </div>
      <div class="flex gap-2">
        <a href="/messages/compose?to={partner_name_encoded}" class="bg-gray-100 text-black px-3 py-2 rounded-lg text-xs font-medium hover:bg-gray-200 transition-colors"><i class="fa fa-pencil mr-1"></i>新消息</a>
        <button onclick="if(confirm('确定删除与{partner_name}的对话？'))postAction('/messages/{partner_id}/delete')" class="bg-gray-100 text-red-500 px-3 py-2 rounded-lg text-xs font-medium hover:bg-gray-200 transition-colors"><i class="fa fa-trash-o"></i></button>
      </div>
    </div>

    <!-- Chat messages -->
    <div class="bg-white border border-gray-200 rounded-lg p-5 mb-4 fade-in max-h-[500px] overflow-y-auto" id="chatBox">
      {messages_html}
    </div>

    <!-- Reply form -->
    <div class="bg-white border border-gray-200 rounded-lg p-5 fade-in">
      <form method="POST" action="/messages/{partner_id}/reply" accept-charset="UTF-8">
        <div class="flex gap-3">
          <textarea name="content" rows="2" required placeholder="输入消息..."
            class="flex-1 border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-none"></textarea>
          <button type="submit" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors flex-shrink-0">
            <i class="fa fa-paper-plane mr-1"></i>发送
          </button>
        </div>
      </form>
    </div>"#,
        partner_id = partner_id,
        partner_name = html_escape(partner_name),
        partner_name_encoded = url_encode(partner_name),
        partner_avatar_html = partner_avatar_html,
        messages_html = messages_html,
    );

    page_with_sidebar(&format!("与{}的对话", partner_name), &main, Some(user), "", "home")
}

// =====================================================================
// Message: Compose
// =====================================================================

pub fn render_compose(user: &User, to: Option<&str>) -> String {
    let to_value = to.map(|t| html_escape(t)).unwrap_or_default();

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/messages" class="hover:text-black">消息</a> <i class="fa fa-angle-right"></i> <span class="text-black">写新消息</span></div>

    <div class="bg-white border border-gray-200 rounded-lg p-6 fade-in">
      <h2 class="text-xl font-semibold mb-6">写新消息</h2>
      <form method="POST" action="/messages/send" accept-charset="UTF-8">
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">收件人</label>
          <input type="text" name="to" value="{to_value}" required placeholder="输入用户名"
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors">
        </div>
        <div class="mb-5">
          <label class="block text-sm font-medium mb-2">内容</label>
          <textarea name="content" rows="5" required placeholder="输入消息内容..."
            class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors resize-y"></textarea>
        </div>
        <div class="flex gap-3">
          <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
            <i class="fa fa-paper-plane mr-1"></i>发送
          </button>
          <a href="/messages" class="bg-gray-100 text-black px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors">取消</a>
        </div>
      </form>
    </div>"#,
        to_value = to_value,
    );

    page_with_sidebar("写新消息", &main, Some(user), "", "home")
}

// =====================================================================
// URL encode helper
// =====================================================================

fn url_encode(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

// =====================================================================
// About Page — Tech Stack, Features & Changelog
// =====================================================================

pub fn render_about() -> String {
    let site_name = crate::site_config::site_name();

    let tech_stack = r##"
    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
      <!-- Core Framework -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-orange-100 text-orange-600 rounded-lg flex items-center justify-center"><i class="fa fa-cog text-lg"></i></div>
          <h3 class="font-semibold">核心框架</h3>
        </div>
        <ul class="text-sm text-gray-600 space-y-2">
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-orange-400 rounded-full flex-shrink-0"></span><strong>Axum 0.8</strong> — 高性能异步 Web 框架</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-orange-400 rounded-full flex-shrink-0"></span><strong>Tokio 1</strong> — 异步运行时</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-orange-400 rounded-full flex-shrink-0"></span><strong>Tower / Tower-HTTP</strong> — 中间件与静态文件</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-orange-400 rounded-full flex-shrink-0"></span><strong>Rust 2021 Edition</strong> — 安全、高性能系统语言</li>
        </ul>
      </div>

      <!-- Database -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-blue-100 text-blue-600 rounded-lg flex items-center justify-center"><i class="fa fa-database text-lg"></i></div>
          <h3 class="font-semibold">数据层</h3>
        </div>
        <ul class="text-sm text-gray-600 space-y-2">
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-blue-400 rounded-full flex-shrink-0"></span><strong>SQLite</strong> — 零依赖嵌入式数据库</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-blue-400 rounded-full flex-shrink-0"></span><strong>SQLx 0.8</strong> — 编译期 SQL 检查</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-blue-400 rounded-full flex-shrink-0"></span><strong>自动迁移</strong> — 启动时自动建表与结构升级</li>
        </ul>
      </div>

      <!-- Frontend -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-purple-100 text-purple-600 rounded-lg flex items-center justify-center"><i class="fa fa-paint-brush text-lg"></i></div>
          <h3 class="font-semibold">前端技术</h3>
        </div>
        <ul class="text-sm text-gray-600 space-y-2">
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-purple-400 rounded-full flex-shrink-0"></span><strong>Tailwind CSS</strong> — 原子化 CSS 框架</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-purple-400 rounded-full flex-shrink-0"></span><strong>Font Awesome</strong> — 图标库</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-purple-400 rounded-full flex-shrink-0"></span><strong>Multiavatar</strong> — 动态头像生成</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-purple-400 rounded-full flex-shrink-0"></span><strong>服务端渲染 (SSR)</strong> — 全页面后端直出 HTML</li>
        </ul>
      </div>

      <!-- Content Processing -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-green-100 text-green-600 rounded-lg flex items-center justify-center"><i class="fa fa-file-code-o text-lg"></i></div>
          <h3 class="font-semibold">内容处理</h3>
        </div>
        <ul class="text-sm text-gray-600 space-y-2">
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-green-400 rounded-full flex-shrink-0"></span><strong>pulldown-cmark 0.13</strong> — Markdown 解析渲染</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-green-400 rounded-full flex-shrink-0"></span><strong>bcrypt 0.17</strong> — 安全密码哈希</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-green-400 rounded-full flex-shrink-0"></span><strong>UUID v4</strong> — 会话标识生成</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-green-400 rounded-full flex-shrink-0"></span><strong>reqwest</strong> — AI API HTTP 客户端</li>
        </ul>
      </div>

      <!-- Observability -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-red-100 text-red-600 rounded-lg flex items-center justify-center"><i class="fa fa-line-chart text-lg"></i></div>
          <h3 class="font-semibold">可观测性</h3>
        </div>
        <ul class="text-sm text-gray-600 space-y-2">
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-red-400 rounded-full flex-shrink-0"></span><strong>tracing</strong> — 结构化日志</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-red-400 rounded-full flex-shrink-0"></span><strong>tracing-subscriber</strong> — 日志输出与过滤</li>
        </ul>
      </div>

      <!-- Serialization -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-10 h-10 bg-yellow-100 text-yellow-600 rounded-lg flex items-center justify-center"><i class="fa fa-exchange text-lg"></i></div>
          <h3 class="font-semibold">数据交互</h3>
        </div>
        <ul class="text-sm text-gray-600 space-y-2">
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-yellow-400 rounded-full flex-shrink-0"></span><strong>serde / serde_json</strong> — JSON 序列化</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-yellow-400 rounded-full flex-shrink-0"></span><strong>chrono 0.4</strong> — 日期时间处理</li>
          <li class="flex items-center gap-2"><span class="w-1.5 h-1.5 bg-yellow-400 rounded-full flex-shrink-0"></span><strong>multipart</strong> — 文件上传处理</li>
        </ul>
      </div>
    </div>"##;

    let features = r##"
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
      <!-- User System -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 bg-indigo-100 text-indigo-600 rounded-lg flex items-center justify-center"><i class="fa fa-users"></i></div>
          <h3 class="font-semibold">用户体系</h3>
        </div>
        <ul class="text-xs text-gray-600 space-y-1.5">
          <li><i class="fa fa-check text-green-500 mr-1"></i>用户注册 / 登录 / 登出</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>Cookie 会话管理 (SQLite)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>邀请码注册</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>用户组：管理员 / 版主 / 会员</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>个人资料编辑</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>头像上传 / 删除</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>自定义头衔 / 称号 / 称号颜色</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>密码修改</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>积分等级体系</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>用户悬浮卡片</li>
        </ul>
      </div>

      <!-- Forum Core -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 bg-teal-100 text-teal-600 rounded-lg flex items-center justify-center"><i class="fa fa-comments"></i></div>
          <h3 class="font-semibold">论坛核心</h3>
        </div>
        <ul class="text-xs text-gray-600 space-y-1.5">
          <li><i class="fa fa-check text-green-500 mr-1"></i>版块管理 (创建 / 编辑 / 删除 / 排序)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>帖子发布 / 编辑 / 删除</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>回复 / 引用回复</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>楼层显示 (楼主标记)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>帖子置顶 / 精华 / 关闭</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>浏览量统计</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>分页导航</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>搜索功能</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>最新 / 热门 / 精华标签页</li>
        </ul>
      </div>

      <!-- Editor & Content -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 bg-pink-100 text-pink-600 rounded-lg flex items-center justify-center"><i class="fa fa-pencil"></i></div>
          <h3 class="font-semibold">编辑器与内容</h3>
        </div>
        <ul class="text-xs text-gray-600 space-y-1.5">
          <li><i class="fa fa-check text-green-500 mr-1"></i>Markdown 富文本渲染</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>编辑器工具栏</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>代码块 / 表格 / 任务列表</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>[quote] 引用语法</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>Emoji 表情选择器 (60+)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>行内编辑 (回复直接编辑)</li>
        </ul>
      </div>

      <!-- Interaction -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 bg-amber-100 text-amber-600 rounded-lg flex items-center justify-center"><i class="fa fa-bell"></i></div>
          <h3 class="font-semibold">互动系统</h3>
        </div>
        <ul class="text-xs text-gray-600 space-y-1.5">
          <li><i class="fa fa-check text-green-500 mr-1"></i>通知系统 (回复 / 引用 / 消息)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>通知铃铛 + 下拉面板</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>私聊消息 (收件箱 / 对话)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>每日签到 + 积分奖励</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>积分排行榜</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>举报功能</li>
        </ul>
      </div>

      <!-- Admin Panel -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 bg-gray-800 text-white rounded-lg flex items-center justify-center"><i class="fa fa-shield"></i></div>
          <h3 class="font-semibold">管理后台</h3>
        </div>
        <ul class="text-xs text-gray-600 space-y-1.5">
          <li><i class="fa fa-check text-green-500 mr-1"></i>独立后台布局 (侧边栏导航)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>仪表盘 (8 项统计)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>帖子 / 版块 / 用户管理</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>举报管理 (处理 / 驳回 / 备注)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>黑名单 / IP 封禁</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>禁言 (限时 / 永久)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>邀请码管理</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>AI 安全审查</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>系统设置 (站点 / 注册 / 积分 / 上传)</li>
        </ul>
      </div>

      <!-- API -->
      <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
        <div class="flex items-center gap-3 mb-3">
          <div class="w-9 h-9 bg-cyan-100 text-cyan-600 rounded-lg flex items-center justify-center"><i class="fa fa-plug"></i></div>
          <h3 class="font-semibold">JSON API</h3>
        </div>
        <ul class="text-xs text-gray-600 space-y-1.5">
          <li><i class="fa fa-check text-green-500 mr-1"></i>帖子列表 / 论坛列表 / 统计</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>搜索 API</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>认证 API (登录 / 注册 / 登出)</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>发帖 / 回复 API</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>用户卡片 API</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>通知 API</li>
          <li><i class="fa fa-check text-green-500 mr-1"></i>签到 / 排行 API</li>
        </ul>
      </div>
    </div>"##;

    let changelog = r##"
    <div class="space-y-0">
      <!-- v1.5.0 -->
      <div class="relative pl-8 pb-8 border-l-2 border-gray-200 last:border-l-0">
        <div class="absolute left-[-9px] top-0 w-4 h-4 bg-black rounded-full border-2 border-white"></div>
        <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
          <div class="flex items-center gap-3 mb-3">
            <span class="bg-black text-white px-2.5 py-0.5 rounded text-xs font-bold">v1.5.0</span>
            <span class="text-xs text-gray-400">2026-04-06</span>
          </div>
          <h4 class="font-medium text-sm mb-2">系统设置与邀请码</h4>
          <ul class="text-xs text-gray-600 space-y-1">
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>邀请码注册系统（生成、验证、使用次数限制）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>管理后台邀请码管理页面</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>站点信息动态配置（名称/描述/关键词/页脚）</li>
            <li><span class="inline-block w-14 text-green-600 font-medium">优化</span>所有后台操作增加成功/失败反馈提示</li>
            <li><span class="inline-block w-14 text-green-600 font-medium">优化</span>移除所有硬编码，站点配置由数据库驱动</li>
          </ul>
        </div>
      </div>

      <!-- v1.4.0 -->
      <div class="relative pl-8 pb-8 border-l-2 border-gray-200">
        <div class="absolute left-[-9px] top-0 w-4 h-4 bg-gray-400 rounded-full border-2 border-white"></div>
        <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
          <div class="flex items-center gap-3 mb-3">
            <span class="bg-gray-700 text-white px-2.5 py-0.5 rounded text-xs font-bold">v1.4.0</span>
            <span class="text-xs text-gray-400">2026-04-05</span>
          </div>
          <h4 class="font-medium text-sm mb-2">管理后台全面升级</h4>
          <ul class="text-xs text-gray-600 space-y-1">
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>独立后台布局（左侧边栏导航）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>帖子管理页面（置顶/精华/关闭/删除）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>举报系统（前台举报 + 后台处理）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>黑名单 / IP 封禁管理</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>用户禁言（限时/永久 + 原因记录）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>AI 安全审查（接入 OpenAI 兼容 API）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>系统设置子页面（站点/注册/积分/上传/AI）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>仪表盘扩展（8 项统计 + 最近举报 + 新注册）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>版主权限中间件 (ModeratorUser)</li>
          </ul>
        </div>
      </div>

      <!-- v1.3.0 -->
      <div class="relative pl-8 pb-8 border-l-2 border-gray-200">
        <div class="absolute left-[-9px] top-0 w-4 h-4 bg-gray-400 rounded-full border-2 border-white"></div>
        <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
          <div class="flex items-center gap-3 mb-3">
            <span class="bg-gray-600 text-white px-2.5 py-0.5 rounded text-xs font-bold">v1.3.0</span>
            <span class="text-xs text-gray-400">2026-04-04</span>
          </div>
          <h4 class="font-medium text-sm mb-2">通知与互动</h4>
          <ul class="text-xs text-gray-600 space-y-1">
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>通知系统（回复/引用/消息三种类型）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>头部通知铃铛 + 下拉面板</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>行内帖子编辑（回复框直接编辑）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>编辑器工具栏（加粗/斜体/代码/引用等）</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>Emoji 表情选择器 (60+)</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>[quote=username] 引用渲染</li>
          </ul>
        </div>
      </div>

      <!-- v1.2.0 -->
      <div class="relative pl-8 pb-8 border-l-2 border-gray-200">
        <div class="absolute left-[-9px] top-0 w-4 h-4 bg-gray-400 rounded-full border-2 border-white"></div>
        <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
          <div class="flex items-center gap-3 mb-3">
            <span class="bg-gray-500 text-white px-2.5 py-0.5 rounded text-xs font-bold">v1.2.0</span>
            <span class="text-xs text-gray-400">2026-04-03</span>
          </div>
          <h4 class="font-medium text-sm mb-2">签到与社交</h4>
          <ul class="text-xs text-gray-600 space-y-1">
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>每日签到 + 积分奖励</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>积分排行榜 (Top 10)</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>私聊消息系统</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>头像上传 / 删除</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>用户悬浮卡片</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>友情链接模块</li>
            <li><span class="inline-block w-14 text-green-600 font-medium">优化</span>侧边栏统一布局</li>
          </ul>
        </div>
      </div>

      <!-- v1.1.0 -->
      <div class="relative pl-8 pb-8 border-l-2 border-gray-200">
        <div class="absolute left-[-9px] top-0 w-4 h-4 bg-gray-400 rounded-full border-2 border-white"></div>
        <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
          <div class="flex items-center gap-3 mb-3">
            <span class="bg-gray-400 text-white px-2.5 py-0.5 rounded text-xs font-bold">v1.1.0</span>
            <span class="text-xs text-gray-400">2026-04-02</span>
          </div>
          <h4 class="font-medium text-sm mb-2">用户体系完善</h4>
          <ul class="text-xs text-gray-600 space-y-1">
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>个人资料编辑页</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>自定义头衔 / 称号系统</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>用户积分等级体系</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>我的帖子页面</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>用户资料查看页</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>JSON API 接口</li>
          </ul>
        </div>
      </div>

      <!-- v1.0.0 -->
      <div class="relative pl-8">
        <div class="absolute left-[-9px] top-0 w-4 h-4 bg-gray-300 rounded-full border-2 border-white"></div>
        <div class="bg-white border border-gray-200 rounded-xl p-5 shadow-sm">
          <div class="flex items-center gap-3 mb-3">
            <span class="bg-gray-300 text-gray-700 px-2.5 py-0.5 rounded text-xs font-bold">v1.0.0</span>
            <span class="text-xs text-gray-400">2026-04-01</span>
          </div>
          <h4 class="font-medium text-sm mb-2">初始发布</h4>
          <ul class="text-xs text-gray-600 space-y-1">
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>用户注册 / 登录 / 登出</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>版块 / 帖子 / 回复 CRUD</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>Markdown 渲染</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>基础管理后台</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>Cookie 会话认证</li>
            <li><span class="inline-block w-14 text-blue-600 font-medium">新增</span>SQLite 数据库 + 自动迁移</li>
          </ul>
        </div>
      </div>
    </div>"##;

    let content = format!(r##"
    <div class="container mx-auto px-4 py-10 max-w-5xl">
      <!-- Hero -->
      <div class="text-center mb-12">
        <div class="inline-flex items-center gap-2 bg-black text-white px-4 py-1.5 rounded-full text-xs font-medium mb-4">
          <i class="fa fa-code-fork"></i> {site_name}
        </div>
        <h1 class="text-3xl font-bold mb-3">关于 {site_name}</h1>
        <p class="text-gray-500 max-w-2xl mx-auto">基于 Rust + Axum + SQLite 构建的高性能论坛系统，全服务端渲染、零外部依赖、单二进制部署。</p>
      </div>

      <!-- Tech Stack -->
      <div class="mb-12">
        <h2 class="text-xl font-semibold mb-5 flex items-center gap-2"><i class="fa fa-layer-group text-gray-400"></i> 技术栈</h2>
        {tech_stack}
      </div>

      <!-- Features -->
      <div class="mb-12">
        <h2 class="text-xl font-semibold mb-5 flex items-center gap-2"><i class="fa fa-star text-gray-400"></i> 功能特性</h2>
        {features}
      </div>

      <!-- Architecture highlights -->
      <div class="mb-12 bg-white border border-gray-200 rounded-xl p-6 shadow-sm">
        <h2 class="text-xl font-semibold mb-4 flex items-center gap-2"><i class="fa fa-server text-gray-400"></i> 架构亮点</h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-4 text-sm text-gray-600">
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 bg-orange-100 text-orange-600 rounded-lg flex items-center justify-center flex-shrink-0"><i class="fa fa-bolt"></i></div>
            <div><strong>单二进制部署</strong><br>编译后只有一个可执行文件，无需安装运行时或数据库</div>
          </div>
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 bg-blue-100 text-blue-600 rounded-lg flex items-center justify-center flex-shrink-0"><i class="fa fa-shield"></i></div>
            <div><strong>内存安全</strong><br>Rust 编译期保证无数据竞争、无空指针、无缓冲区溢出</div>
          </div>
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 bg-green-100 text-green-600 rounded-lg flex items-center justify-center flex-shrink-0"><i class="fa fa-rocket"></i></div>
            <div><strong>极致性能</strong><br>异步 I/O + 零拷贝，单机轻松承载万级并发连接</div>
          </div>
          <div class="flex items-start gap-3">
            <div class="w-8 h-8 bg-purple-100 text-purple-600 rounded-lg flex items-center justify-center flex-shrink-0"><i class="fa fa-cube"></i></div>
            <div><strong>全 SSR 渲染</strong><br>无前端框架依赖，后端直出 HTML，首屏秒开</div>
          </div>
        </div>
      </div>

      <!-- Changelog -->
      <div class="mb-12">
        <h2 class="text-xl font-semibold mb-5 flex items-center gap-2"><i class="fa fa-history text-gray-400"></i> 版本记录</h2>
        {changelog}
      </div>

      <!-- Footer -->
      <div class="text-center text-sm text-gray-400 pt-8 border-t border-gray-200">
        <p>Made with <i class="fa fa-heart text-red-400"></i> using Rust + Axum + SQLite</p>
        <p class="mt-1">Copyright &copy; 2026 {site_name}</p>
      </div>
    </div>"##,
        site_name = html_escape(&site_name),
        tech_stack = tech_stack,
        features = features,
        changelog = changelog,
    );

    // About page uses a simpler layout without sidebar
    let full = format!(r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>关于 | {site_name}</title>
  <script src="/static/css/tailwind.js"></script>
  <link href="/static/css/font-awesome.min.css" rel="stylesheet">
  <script>
    tailwind.config = {{
      theme: {{
        extend: {{
          colors: {{
            primary: '#000000',
            secondary: '#666666',
            muted: '#f5f5f5',
          }},
          fontFamily: {{
            sans: ['Inter', 'system-ui', 'sans-serif'],
          }},
        }},
      }}
    }}
  </script>
  <style type="text/tailwindcss">
    @layer utilities {{
      .fade-in {{ animation: fadeIn 0.3s ease; }}
      @keyframes fadeIn {{ from {{ opacity:0; transform:translateY(8px) }} to {{ opacity:1; transform:translateY(0) }} }}
    }}
  </style>
</head>
<body class="text-black min-h-screen" style="background-color:#fafafa;background-image:linear-gradient(to right, #e7e5e4 1px, transparent 1px),linear-gradient(to bottom, #e7e5e4 1px, transparent 1px);background-size:40px 40px;">
<!-- Simple header -->
<header class="sticky top-0 z-50 bg-white border-b border-gray-200 backdrop-blur-sm bg-opacity-90">
  <div class="container mx-auto px-4 py-4 flex items-center justify-between">
    <a href="/" class="flex items-center gap-2 font-semibold text-lg">
      <i class="fa fa-comments"></i>
      {site_name}
    </a>
    <nav class="flex items-center gap-4 text-sm">
      <a href="/" class="text-gray-500 hover:text-black transition-colors">首页</a>
      <a href="/forums" class="text-gray-500 hover:text-black transition-colors">版块</a>
    </nav>
  </div>
</header>

<main class="fade-in">
{content}
</main>
</body>
</html>"##,
        site_name = html_escape(&site_name),
        content = content,
    );

    full
}

// =====================================================================
// Admin Backup Page
// =====================================================================

pub fn render_admin_backup(backups: &[(String, String, String, String)]) -> String {
    let backup_rows = if backups.is_empty() {
        r#"<tr><td colspan="4" class="px-4 py-8 text-center text-gray-400">暂无备份文件</td></tr>"#.to_string()
    } else {
        backups
            .iter()
            .map(|(filename, size, created_at, raw_name)| {
                format!(
                    r#"<tr class="border-b border-gray-50 hover:bg-gray-50">
        <td class="px-4 py-3 text-sm font-mono">{filename}</td>
        <td class="px-4 py-3 text-sm text-gray-600">{size}</td>
        <td class="px-4 py-3 text-sm text-gray-600">{created_at}</td>
        <td class="px-4 py-3 text-sm space-x-2">
          <a href="/admin/backup/download/{raw_name}" class="text-blue-600 hover:text-blue-800"><i class="fa fa-download mr-1"></i>下载</a>
          <form method="POST" action="/admin/backup/delete/{raw_name}" class="inline" onsubmit="return confirm('确定要删除此备份吗？')"><button type="submit" class="text-red-600 hover:text-red-800"><i class="fa fa-trash-o mr-1"></i>删除</button></form>
        </td>
      </tr>"#,
                    filename = html_escape(filename),
                    size = size,
                    created_at = created_at,
                    raw_name = html_escape(raw_name),
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    };

    let content = format!(
        r#"
<h1 class="text-2xl font-bold mb-6">数据备份与恢复</h1>

<!-- Create Backup -->
<div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 mb-6 fade-in">
  <h3 class="font-semibold mb-3"><i class="fa fa-database mr-2"></i>创建备份</h3>
  <p class="text-sm text-gray-500 mb-4">创建完整的数据库和头像文件备份。备份文件为 ZIP 格式，包含数据库文件、清单文件和所有用户头像。</p>
  <form method="POST" action="/admin/backup/create">
    <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
      <i class="fa fa-plus mr-1"></i>立即备份
    </button>
  </form>
</div>

<!-- Restore Backup -->
<div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 mb-6 fade-in">
  <h3 class="font-semibold mb-3"><i class="fa fa-upload mr-2"></i>恢复数据</h3>
  <div class="bg-yellow-50 border border-yellow-200 rounded-lg p-3 mb-4 text-sm text-yellow-700">
    <i class="fa fa-exclamation-triangle mr-1"></i>
    恢复将覆盖所有数据，恢复前会自动创建安全备份。恢复后需要手动重启服务器。
  </div>
  <form id="restoreForm" method="POST" action="/admin/backup/restore" enctype="multipart/form-data">
    <div class="flex items-center gap-4">
      <input type="file" name="backup" accept=".zip" required class="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-gray-100 file:text-gray-700 hover:file:bg-gray-200">
      <button type="submit" class="bg-red-600 text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-red-700 transition-colors whitespace-nowrap">
        <i class="fa fa-undo mr-1"></i>上传并恢复
      </button>
    </div>
  </form>
</div>

<!-- Backup List -->
<div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6 fade-in">
  <h3 class="font-semibold mb-4"><i class="fa fa-list mr-2"></i>备份列表</h3>
  <div class="overflow-x-auto">
    <table class="w-full">
      <thead>
        <tr class="border-b border-gray-200 text-left text-sm text-gray-500">
          <th class="px-4 py-2 font-medium">文件名</th>
          <th class="px-4 py-2 font-medium">大小</th>
          <th class="px-4 py-2 font-medium">创建时间</th>
          <th class="px-4 py-2 font-medium">操作</th>
        </tr>
      </thead>
      <tbody>
        {backup_rows}
      </tbody>
    </table>
  </div>
</div>

<script>
document.getElementById('restoreForm').addEventListener('submit', function(e) {{
  if (!confirm('确定要恢复数据吗？当前数据将被覆盖，恢复前会自动创建安全备份。')) {{
    e.preventDefault();
  }}
}});
</script>"#,
        backup_rows = backup_rows,
    );

    admin_layout("数据备份", "backup", &content)
}

// =====================================================================
// Terms of Service / Privacy Policy / Contact Us
// =====================================================================

pub fn render_terms() -> String {
    let site_name = crate::site_config::site_name();

    let content = format!(r#"
<div class="container mx-auto px-4 py-8 max-w-5xl">
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-8 fade-in">
    <h1 class="text-2xl font-bold mb-2">使用条款</h1>
    <p class="text-sm text-gray-400 mb-8">最后更新：2026年4月6日</p>

    <div class="prose max-w-none text-sm text-gray-700 space-y-6 leading-relaxed">

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">一、总则</h2>
        <p>欢迎您使用{site_name}（以下简称"本站"）。在使用本站之前，请您仔细阅读并充分理解本使用条款。您一旦注册、登录或使用本站服务，即视为您已充分理解并同意接受本条款的全部内容。</p>
        <p>本站保留随时修改本条款的权利，修改后的条款将在本页面公布。继续使用本站即视为同意修改后的条款。</p>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">二、账户注册与管理</h2>
        <ul class="list-disc pl-5 space-y-1">
          <li>用户在注册时应提供真实、准确、完整的个人信息。</li>
          <li>每位用户仅限注册一个账户，不得冒用他人信息注册。</li>
          <li>用户应妥善保管账户信息和密码，因账户密码保管不善造成的损失由用户自行承担。</li>
          <li>用户不得将账户转让、出售或出借给他人使用。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">三、用户行为规范</h2>
        <p>用户在使用本站时，不得发布以下内容：</p>
        <ul class="list-disc pl-5 space-y-1">
          <li>违反中华人民共和国法律法规的内容；</li>
          <li>危害国家安全、泄露国家秘密的内容；</li>
          <li>散布淫秽、色情、赌博、暴力、凶杀、恐怖内容或教唆犯罪的；</li>
          <li>侮辱或者诽谤他人，侵害他人合法权益的；</li>
          <li>虚假的、骚扰性的、恐吓性的信息；</li>
          <li>未经授权的广告、推广信息或垃圾信息；</li>
          <li>含有恶意代码、病毒或其他有害程序的内容；</li>
          <li>其他违反公序良俗或本站管理规则的内容。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">四、知识产权</h2>
        <p>用户在本站发布的原创内容，其知识产权归原作者所有。用户发布内容即视为授予本站非独家的、免费的、可撤销的使用许可，本站有权在站内展示、管理和维护该内容。</p>
        <p>本站的系统设计、页面布局、标识等属于本站所有，未经许可不得复制或使用。</p>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">五、免责声明</h2>
        <ul class="list-disc pl-5 space-y-1">
          <li>本站不对用户发布的内容承担审查义务，但有权对违规内容进行删除或屏蔽。</li>
          <li>用户因使用本站服务而产生的任何直接或间接损失，本站不承担赔偿责任。</li>
          <li>因不可抗力、系统故障等原因导致服务中断的，本站不承担责任。</li>
          <li>本站不对通过本站获取的第三方信息的准确性和完整性做出保证。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">六、违规处理</h2>
        <p>对于违反本条款的用户，本站有权视情节轻重采取以下措施：</p>
        <ul class="list-disc pl-5 space-y-1">
          <li>警告提醒；</li>
          <li>删除违规内容；</li>
          <li>禁言处理；</li>
          <li>封禁账户；</li>
          <li>追究法律责任（如涉嫌违法）。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">七、其他</h2>
        <p>本条款的解释、效力及争议解决均适用中华人民共和国法律。如本条款与相关法律法规相抵触，以法律法规为准。</p>
      </div>

    </div>
  </div>
</div>"#,
        site_name = html_escape(&site_name),
    );

    layout("使用条款", None, &content, "home")
}

pub fn render_privacy() -> String {
    let site_name = crate::site_config::site_name();

    let content = format!(r#"
<div class="container mx-auto px-4 py-8 max-w-5xl">
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-8 fade-in">
    <h1 class="text-2xl font-bold mb-2">隐私政策</h1>
    <p class="text-sm text-gray-400 mb-8">最后更新：2026年4月6日</p>

    <div class="prose max-w-none text-sm text-gray-700 space-y-6 leading-relaxed">

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">一、信息收集</h2>
        <p>在您使用{site_name}（以下简称"本站"）服务的过程中，我们会收集以下信息：</p>
        <ul class="list-disc pl-5 space-y-1">
          <li><strong>注册信息：</strong>用户名、密码（加密存储）、邮箱（如提供）。</li>
          <li><strong>内容信息：</strong>您发布的帖子、回复、私信、头像等。</li>
          <li><strong>日志信息：</strong>访问时间、页面浏览记录等服务器日志。</li>
          <li><strong>Cookie 信息：</strong>我们使用 Cookie 维持您的登录状态（Session Token），不用于广告追踪。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">二、信息使用</h2>
        <p>我们收集的信息仅用于以下目的：</p>
        <ul class="list-disc pl-5 space-y-1">
          <li>提供、维护和改进本站服务；</li>
          <li>账户身份验证和安全管理；</li>
          <li>社区内容管理和违规处理；</li>
          <li>与您沟通服务相关事项。</li>
        </ul>
        <p>我们不会将您的个人信息出售、出租或交易给任何第三方。</p>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">三、信息存储与安全</h2>
        <ul class="list-disc pl-5 space-y-1">
          <li>您的数据存储在本站服务器中，我们采取合理的技术措施保护数据安全。</li>
          <li>密码使用 bcrypt 算法单向加密存储，任何人均无法查看您的原始密码。</li>
          <li>我们限制员工和系统对个人数据的访问权限。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">四、信息共享</h2>
        <p>除以下情况外，我们不会与任何第三方共享您的个人信息：</p>
        <ul class="list-disc pl-5 space-y-1">
          <li>事先获得您的明确同意；</li>
          <li>根据法律法规或政府主管部门的强制性要求；</li>
          <li>为维护本站及其他用户的合法权益。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">五、用户权利</h2>
        <p>您对自己的个人信息享有以下权利：</p>
        <ul class="list-disc pl-5 space-y-1">
          <li><strong>访问权：</strong>您可以随时登录账户查看自己的个人信息和发布内容。</li>
          <li><strong>修改权：</strong>您可以编辑个人资料和发布的内容。</li>
          <li><strong>删除权：</strong>您可以联系管理员申请删除您的账户及相关数据。</li>
        </ul>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">六、Cookie 政策</h2>
        <p>本站使用 Cookie 仅为维持用户登录状态（会话标识）。我们不使用任何第三方分析或广告追踪 Cookie。您可以通过浏览器设置禁用 Cookie，但这可能导致无法正常登录使用本站。</p>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">七、未成年人保护</h2>
        <p>本站不面向未满 14 周岁的未成年人提供服务。如果我们发现未满 14 周岁的未成年人未经监护人同意注册使用本站，我们将及时删除其个人信息。</p>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">八、隐私政策更新</h2>
        <p>我们可能会不时更新本隐私政策。更新后的政策将在本页面发布并修改"最后更新"日期。继续使用本站即视为同意更新后的隐私政策。</p>
      </div>

      <div>
        <h2 class="text-lg font-semibold text-gray-900 mb-2">九、联系我们</h2>
        <p>如您对本隐私政策有任何疑问，请通过以下方式联系我们：</p>
        <p>邮箱：<a href="mailto:admin@example.com" class="text-blue-600 hover:underline">admin@example.com</a></p>
      </div>

    </div>
  </div>
</div>"#,
        site_name = html_escape(&site_name),
    );

    layout("隐私政策", None, &content, "home")
}

pub fn render_contact() -> String {
    let site_name = crate::site_config::site_name();

    let content = format!(r#"
<div class="container mx-auto px-4 py-8 max-w-5xl">
  <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-8 fade-in">
    <h1 class="text-2xl font-bold mb-2">联系我们</h1>
    <p class="text-sm text-gray-400 mb-8">如有任何问题或建议，欢迎通过以下方式联系我们</p>

    <div class="space-y-6">

      <div class="flex items-start gap-4 p-5 bg-gray-50 rounded-xl">
        <div class="w-12 h-12 bg-blue-100 text-blue-600 rounded-xl flex items-center justify-center flex-shrink-0">
          <i class="fa fa-envelope text-xl"></i>
        </div>
        <div>
          <h3 class="font-semibold text-gray-900 mb-1">电子邮箱</h3>
          <p class="text-sm text-gray-500 mb-2">工作日通常在 24 小时内回复</p>
          <a href="mailto:admin@example.com" class="text-blue-600 hover:underline font-medium">admin@example.com</a>
        </div>
      </div>

      <div class="flex items-start gap-4 p-5 bg-gray-50 rounded-xl">
        <div class="w-12 h-12 bg-green-100 text-green-600 rounded-xl flex items-center justify-center flex-shrink-0">
          <i class="fa fa-comments text-xl"></i>
        </div>
        <div>
          <h3 class="font-semibold text-gray-900 mb-1">社区反馈</h3>
          <p class="text-sm text-gray-500 mb-2">在社区中发帖提出您的建议或反馈</p>
          <a href="/forums" class="text-blue-600 hover:underline font-medium">前往版块 &rarr;</a>
        </div>
      </div>

      <div class="flex items-start gap-4 p-5 bg-gray-50 rounded-xl">
        <div class="w-12 h-12 bg-purple-100 text-purple-600 rounded-xl flex items-center justify-center flex-shrink-0">
          <i class="fa fa-shield text-xl"></i>
        </div>
        <div>
          <h3 class="font-semibold text-gray-900 mb-1">举报与申诉</h3>
          <p class="text-sm text-gray-500 mb-2">发现违规内容或需要申诉，请使用站内举报功能或发送邮件</p>
          <p class="text-sm text-gray-600">在帖子或回复旁边点击"举报"按钮即可提交举报</p>
        </div>
      </div>

    </div>

    <div class="mt-8 pt-6 border-t border-gray-100 text-center">
      <p class="text-sm text-gray-400">{site_name} 团队</p>
    </div>

  </div>
</div>"#,
        site_name = html_escape(&site_name),
    );

    layout("联系我们", None, &content, "home")
}

// =====================================================================
// Admin Login Logs Page
// =====================================================================

pub fn render_admin_login_logs(logs: &[LoginLogRow], page: i64, total_pages: i64) -> String {
    let rows = if logs.is_empty() {
        r#"<tr><td colspan="7" class="px-4 py-8 text-center text-gray-400">暂无登录记录</td></tr>"#.to_string()
    } else {
        logs.iter().map(|l| {
            let status_cls = if l.success == 1 { "text-green-600" } else { "text-red-500" };
            let status_text = if l.success == 1 { "成功" } else { "失败" };
            let ua_short = if l.user_agent.len() > 50 {
                html_escape(&l.user_agent[..50])
            } else {
                html_escape(&l.user_agent)
            };
            let time_short = l.created_at.chars().take(19).collect::<String>();
            format!(r#"<tr class="border-b border-gray-50 item-hover">
        <td class="px-4 py-3 text-sm">{username}</td>
        <td class="px-4 py-3 text-xs font-mono text-gray-500">{ip}</td>
        <td class="px-4 py-3 text-xs text-gray-500" title="{ua_full}">{ua_short}</td>
        <td class="px-4 py-3 text-sm {status_cls}">{status_text}</td>
        <td class="px-4 py-3 text-xs text-gray-400">{time}</td>
      </tr>"#,
                username = if l.user_id > 0 {
                    format!(r#"<a href="/admin/login-logs?user_id={}" class="hover:underline">{}</a>"#, l.user_id, html_escape(&l.username))
                } else {
                    html_escape(&l.username)
                },
                ip = html_escape(&l.ip),
                ua_short = ua_short,
                ua_full = html_escape(&l.user_agent),
                status_cls = status_cls,
                status_text = status_text,
                time = time_short,
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let pagination = if total_pages > 1 {
        let prev_cls = if page > 1 { "" } else { "opacity-50 pointer-events-none" };
        let next_cls = if page < total_pages { "" } else { "opacity-50 pointer-events-none" };
        format!(r#"
<div class="flex items-center justify-between mt-4 text-sm">
  <span class="text-gray-400">第 {page} / {total_pages} 页</span>
  <div class="flex gap-2">
    <a href="/admin/login-logs?page={prev}" class="px-3 py-1.5 border rounded {prev_cls}">上一页</a>
    <a href="/admin/login-logs?page={next}" class="px-3 py-1.5 border rounded {next_cls}">下一页</a>
  </div>
</div>"#,
            page = page,
            total_pages = total_pages,
            prev = (page - 1).max(1),
            next = (page + 1).min(total_pages),
        )
    } else {
        String::new()
    };

    let content = format!(r#"
<h1 class="text-2xl font-bold mb-6">登录日志</h1>
<div class="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden fade-in">
  <table class="w-full">
    <thead class="bg-gray-50 text-xs text-gray-500"><tr>
      <th class="px-4 py-3 text-left">用户</th>
      <th class="px-4 py-3 text-left">IP</th>
      <th class="px-4 py-3 text-left">浏览器</th>
      <th class="px-4 py-3 text-left">状态</th>
      <th class="px-4 py-3 text-left">时间</th>
    </tr></thead>
    <tbody>{rows}</tbody>
  </table>
</div>
{pagination}
<div class="mt-4">
  <a href="/admin/login-logs" class="text-sm text-blue-600 hover:underline">查看全部</a>
</div>"#,
        rows = rows,
        pagination = pagination,
    );

    admin_layout("登录日志", "login-logs", &content)
}

// =====================================================================
// AI 共享列表页
// =====================================================================

pub fn render_ai_share_list_page(
    shares: &[AiShareList],
    user: Option<&User>,
    category: &str,
    share_type: &str,
    search: &str,
    page: i64,
    total_pages: i64,
) -> String {
    let categories = [
        ("", "全部"),
        ("programming", "编程"),
        ("finance", "金融"),
        ("office", "办公"),
        ("video", "视频"),
        ("creative", "创意"),
    ];
    let types = [
        ("", "全部类型"),
        ("prompt", "Prompt"),
        ("skill", "Skill"),
    ];

    let category_tabs = categories.iter().map(|(val, label)| {
        let active = *val == category;
        let cls = if active {
            "bg-black text-white px-3 py-1.5 rounded-full text-xs font-medium"
        } else {
            "bg-gray-100 text-gray-600 px-3 py-1.5 rounded-full text-xs font-medium hover:bg-gray-200 transition-colors"
        };
        let href = if val.is_empty() {
            format!("/ai?share_type={}&q={}", share_type, search)
        } else {
            format!("/ai?category={}&share_type={}&q={}", val, share_type, search)
        };
        format!(r#"<a href="{}" class="{}">{}</a>"#, href, cls, label)
    }).collect::<Vec<_>>().join("");

    let type_tabs = types.iter().map(|(val, label)| {
        let active = *val == share_type;
        let cls = if active {
            "bg-black text-white px-3 py-1.5 rounded-full text-xs font-medium"
        } else {
            "bg-gray-100 text-gray-600 px-3 py-1.5 rounded-full text-xs font-medium hover:bg-gray-200 transition-colors"
        };
        let href = if val.is_empty() {
            format!("/ai?category={}&q={}", category, search)
        } else {
            format!("/ai?category={}&share_type={}&q={}", category, val, search)
        };
        format!(r#"<a href="{}" class="{}">{}</a>"#, href, cls, label)
    }).collect::<Vec<_>>().join("");

    let cards = if shares.is_empty() {
        r#"<div class="col-span-full text-center py-12 text-gray-400 text-sm">暂无内容，快来分享你的第一个 AI Prompt/Skill 吧</div>"#.to_string()
    } else {
        shares.iter().map(|s| {
            let price_tag = if s.price == 0 {
                r#"<span class="text-xs bg-green-100 text-green-600 px-2 py-0.5 rounded font-medium">免费</span>"#.to_string()
            } else {
                format!(r#"<span class="text-xs bg-orange-100 text-orange-600 px-2 py-0.5 rounded font-medium">{} 积分</span>"#, s.price)
            };
            let type_tag = if s.share_type == "skill" {
                r#"<span class="text-xs bg-purple-100 text-purple-600 px-2 py-0.5 rounded font-medium">Skill</span>"#
            } else {
                r#"<span class="text-xs bg-blue-100 text-blue-600 px-2 py-0.5 rounded font-medium">Prompt</span>"#
            };
            let cat_label = match s.category.as_str() {
                "programming" => "编程",
                "finance" => "金融",
                "office" => "办公",
                "video" => "视频",
                "creative" => "创意",
                _ => &s.category,
            };
            let avatar_span = match crate::templates::avatar_html(&s.avatar, s.user_id, &s.username, "w-5 h-5 text-xs") {
                a if !a.is_empty() => a,
                _ => String::new(),
            };
            format!(r#"<a href="/ai/{id}" class="block bg-white border border-gray-200 rounded-xl p-5 hover:shadow-md transition-shadow">
                <div class="flex items-center gap-2 mb-3">
                  {type_tag}
                  <span class="text-xs bg-gray-100 text-gray-500 px-2 py-0.5 rounded">{cat}</span>
                  {price_tag}
                </div>
                <h3 class="font-medium text-base mb-2 truncate">{title}</h3>
                <p class="text-xs text-gray-500 line-clamp-2 mb-3" style="display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden">{desc}</p>
                <div class="flex items-center justify-between text-xs text-gray-400">
                  <div class="flex items-center gap-1.5">
                    {avatar}
                    <span>{username}</span>
                  </div>
                  <div class="flex items-center gap-3">
                    <span><i class="fa fa-download"></i> {downloads}</span>
                    <span>{time}</span>
                  </div>
                </div>
              </a>"#,
                id = s.id,
                type_tag = type_tag,
                cat = cat_label,
                price_tag = price_tag,
                title = html_escape(&s.title),
                desc = html_escape(&s.description),
                avatar = avatar_span,
                username = html_escape(&s.username),
                downloads = s.download_count,
                time = s.created_at.chars().take(10).collect::<String>(),
            )
        }).collect::<Vec<_>>().join("\n")
    };

    let create_btn = match user {
        Some(_) => r#"<a href="/ai/create" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors"><i class="fa fa-plus mr-1"></i>分享内容</a>"#,
        None => r#"<a href="/auth/login" class="bg-black text-white px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors inline-block"><i class="fa fa-plus mr-1"></i>分享内容</a>"#,
    };

    let pagination = pagination_html(page, total_pages, &format!("/ai?category={}&share_type={}&q={}&", category, share_type, search));

    let main = format!(r#"
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-semibold">AI 共享</h1>
      {create_btn}
    </div>

    <div class="flex flex-wrap items-center gap-2 mb-4">
      {category_tabs}
      <span class="text-gray-300 mx-1">|</span>
      {type_tabs}
    </div>

    <div class="mb-4">
      <form method="get" action="/ai" class="flex gap-2">
        <input type="text" name="q" value="{search}" placeholder="搜索 Prompt / Skill..." class="flex-1 border border-gray-200 rounded-lg px-4 py-2 text-sm outline-none focus:border-black transition-colors">
        <input type="hidden" name="category" value="{category}">
        <input type="hidden" name="share_type" value="{share_type}">
        <button type="submit" class="bg-black text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-gray-800"><i class="fa fa-search"></i></button>
      </form>
    </div>

    <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 mb-6">
      {cards}
    </div>

    {pagination}"#,
        create_btn = create_btn,
        category_tabs = category_tabs,
        type_tabs = type_tabs,
        search = html_escape(search),
        category = category,
        share_type = share_type,
        cards = cards,
        pagination = pagination,
    );

    page_with_sidebar("AI 共享", &main, user, "", "ai")
}

// =====================================================================
// AI 共享详情页
// =====================================================================

pub fn render_ai_share_detail_page(
    share: &AiShare,
    username: &str,
    avatar: &str,
    user: Option<&User>,
    can_view_full: bool,
) -> String {
    let avatar_span = crate::templates::avatar_html(avatar, share.user_id, username, "w-10 h-10 text-sm");
    let price_tag = if share.price == 0 {
        r#"<span class="text-xs bg-green-100 text-green-600 px-2 py-1 rounded font-medium">免费</span>"#.to_string()
    } else {
        format!(r#"<span class="text-xs bg-orange-100 text-orange-600 px-2 py-1 rounded font-medium">{} 积分</span>"#, share.price)
    };
    let type_tag = if share.share_type == "skill" {
        r#"<span class="text-xs bg-purple-100 text-purple-600 px-2 py-1 rounded font-medium">Skill</span>"#
    } else {
        r#"<span class="text-xs bg-blue-100 text-blue-600 px-2 py-1 rounded font-medium">Prompt</span>"#
    };
    let cat_label = match share.category.as_str() {
        "programming" => "编程",
        "finance" => "金融",
        "office" => "办公",
        "video" => "视频",
        "creative" => "创意",
        _ => &share.category,
    };

    let content_html = if can_view_full {
        render_content(&share.content)
    } else {
        let preview = format!("{}\n\n---\n*兑换后查看完整内容...*", html_escape(&share.description));
        render_content(&preview)
    };

    let purchase_btn = if can_view_full {
        String::new()
    } else {
        match user {
            Some(_) => format!(r#"<form method="post" action="/ai/{}/purchase" class="mt-4">
                <button type="submit" class="bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">
                  <i class="fa fa-exchange mr-1"></i>消耗 {} 积分兑换
                </button>
              </form>"#, share.id, share.price),
            None => r#"<a href="/auth/login" class="inline-block bg-black text-white px-6 py-2.5 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors mt-4">登录后兑换</a>"#.to_string(),
        }
    };

    let edit_btn = match user {
        Some(u) if u.id == share.user_id => format!(r#"<a href="/ai/{}/edit" class="text-xs text-gray-400 hover:text-black"><i class="fa fa-edit"></i> 编辑</a>"#, share.id),
        _ => String::new(),
    };
    let delete_btn = match user {
        Some(u) if u.id == share.user_id || u.is_admin() => format!(r#"<form method="post" action="/ai/{}/delete" class="inline" onsubmit="return confirm('确定删除？')"><button type="submit" class="text-xs text-red-400 hover:text-red-600"><i class="fa fa-trash-o"></i> 删除</button></form>"#, share.id),
        _ => String::new(),
    };

    let main = format!(r#"
    <div class="mb-4 text-sm text-gray-500"><a href="/" class="hover:text-black">首页</a> <i class="fa fa-angle-right"></i> <a href="/ai" class="hover:text-black">AI 共享</a> <i class="fa fa-angle-right"></i> <span class="text-black">{title}</span></div>

    <div class="bg-white border border-gray-200 rounded-lg p-6 fade-in">
      <div class="flex items-start justify-between gap-4 mb-4">
        <div>
          <h1 class="text-xl font-semibold">{title}</h1>
          <div class="flex items-center gap-2 mt-2 flex-wrap">
            {type_tag}
            <span class="text-xs bg-gray-100 text-gray-500 px-2 py-1 rounded">{cat}</span>
            {price_tag}
            <span class="text-xs text-gray-400"><i class="fa fa-download"></i> {downloads}</span>
          </div>
        </div>
        <div class="flex items-center gap-3">
          {edit_btn} {delete_btn}
        </div>
      </div>

      <div class="flex items-center gap-3 py-3 border-t border-b border-gray-100">
        {avatar_span}
        <div>
          <a href="/user/{user_id}" class="text-sm font-medium hover:text-black">{username}</a>
          <div class="text-xs text-gray-400">发布于 {time}</div>
        </div>
      </div>

      <div class="py-4">
        <div class="text-sm leading-relaxed markdown-body">{content}</div>
      </div>

      {purchase_btn}
    </div>"#,
        title = html_escape(&share.title),
        type_tag = type_tag,
        cat = cat_label,
        price_tag = price_tag,
        downloads = share.download_count,
        edit_btn = edit_btn,
        delete_btn = delete_btn,
        avatar_span = avatar_span,
        username = html_escape(username),
        user_id = share.user_id,
        time = share.created_at.chars().take(16).collect::<String>(),
        content = content_html,
        purchase_btn = purchase_btn,
    );

    page_with_sidebar(&share.title, &main, user, "", "ai")
}

// =====================================================================
// AI 共享创建/编辑表单
// =====================================================================

pub fn render_ai_share_form(share: Option<&AiShare>) -> String {
    let (title, desc, content, category, share_type, price, form_action, page_title) = match share {
        Some(s) => (
            html_escape(&s.title),
            html_escape(&s.description),
            html_escape(&s.content),
            s.category.as_str(),
            s.share_type.as_str(),
            s.price.to_string(),
            format!("/ai/{}/edit", s.id),
            "编辑内容",
        ),
        None => (
            String::new(),
            String::new(),
            String::new(),
            "",
            "prompt",
            "0".to_string(),
            "/ai/create".to_string(),
            "分享 AI 内容",
        ),
    };

    let cat_options = [
        ("programming", "编程"),
        ("finance", "金融"),
        ("office", "办公"),
        ("video", "视频"),
        ("creative", "创意"),
    ].iter().map(|(val, label)| {
        let selected = if *val == category { " selected" } else { "" };
        format!(r#"<option value="{}"{}>{}</option>"#, val, selected, label)
    }).collect::<Vec<_>>().join("");

    let type_options = [
        ("prompt", "Prompt"),
        ("skill", "Skill"),
    ].iter().map(|(val, label)| {
        let selected = if *val == share_type { " selected" } else { "" };
        format!(r#"<option value="{}"{}>{}</option>"#, val, selected, label)
    }).collect::<Vec<_>>().join("");

    format!(r##"<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>{page_title} | AI 共享</title>
  <script src="/static/css/tailwind.js"></script>
  <link href="/static/css/font-awesome.min.css" rel="stylesheet">
  <style>
    .markdown-body pre {{ background:#1e1e1e; color:#d4d4d4; padding:1em; border-radius:8px; overflow-x:auto; }}
    .markdown-body code {{ background:#f3f4f6; padding:0.15em 0.4em; border-radius:3px; font-size:0.9em; }}
    .markdown-body pre code {{ background:none; padding:0; }}
  </style>
</head>
<body class="bg-gray-50 min-h-screen">
  <div class="container mx-auto px-4 py-8 max-w-2xl">
    <div class="mb-4 text-sm text-gray-500">
      <a href="/ai" class="hover:text-black">AI 共享</a>
      <i class="fa fa-angle-right"></i>
      <span class="text-black">{page_title}</span>
    </div>

    <div class="bg-white border border-gray-200 rounded-lg p-6">
      <h2 class="text-xl font-semibold mb-6">{page_title}</h2>
      <form method="post" action="{form_action}" accept-charset="UTF-8">
        <div class="mb-4">
          <label class="block text-sm font-medium mb-1">标题</label>
          <input type="text" name="title" value="{title}" required
            class="w-full border border-gray-200 rounded-lg px-4 py-2 text-sm outline-none focus:border-black transition-colors" placeholder="给你的 Prompt/Skill 起个名字">
        </div>
        <div class="mb-4">
          <label class="block text-sm font-medium mb-1">简介</label>
          <textarea name="description" rows="3" required
            class="w-full border border-gray-200 rounded-lg px-4 py-2 text-sm outline-none focus:border-black transition-colors resize-y" placeholder="简短描述，方便他人快速了解">{desc}</textarea>
        </div>
        <div class="grid grid-cols-2 gap-4 mb-4">
          <div>
            <label class="block text-sm font-medium mb-1">分类</label>
            <select name="category" required class="w-full border border-gray-200 rounded-lg px-4 py-2 text-sm outline-none focus:border-black">
              {cat_options}
            </select>
          </div>
          <div>
            <label class="block text-sm font-medium mb-1">类型</label>
            <select name="share_type" required class="w-full border border-gray-200 rounded-lg px-4 py-2 text-sm outline-none focus:border-black">
              {type_options}
            </select>
          </div>
        </div>
        <div class="mb-4">
          <label class="block text-sm font-medium mb-1">兑换积分（0 = 免费）</label>
          <input type="number" name="price" value="{price}" min="0"
            class="w-full border border-gray-200 rounded-lg px-4 py-2 text-sm outline-none focus:border-black transition-colors" placeholder="0 表示免费分享">
        </div>
        <div class="mb-6">
          <label class="block text-sm font-medium mb-1">内容（Markdown 格式）</label>
          <textarea name="content" rows="15" required
            class="w-full border border-gray-200 rounded-lg px-4 py-2 text-sm outline-none focus:border-black transition-colors resize-y font-mono leading-relaxed" placeholder="在此粘贴或编写你的 Prompt/Skill 内容，支持 Markdown 格式...">{content}</textarea>
        </div>
        <div class="flex items-center gap-3">
          <button type="submit" class="bg-black text-white px-6 py-2 rounded-lg text-sm font-medium hover:bg-gray-800 transition-colors">提交</button>
          <a href="/ai" class="text-sm text-gray-500 hover:text-black">取消</a>
        </div>
      </form>
    </div>
  </div>
</body>
</html>"##,
        page_title = page_title,
        form_action = form_action,
        title = title,
        desc = desc,
        content = content,
        cat_options = cat_options,
        type_options = type_options,
        price = price,
    )
}

// Render the setup wizard page (independent, does not use layout())
// step: 1=admin account, 2=site settings, 3=complete
// error: error message to display (empty string if none)
pub fn render_setup(step: i32, error: &str) -> String {
    let error_html = if error.is_empty() {
        String::new()
    } else {
        format!(r#"<div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg mb-6">{}</div>"#, html_escape(error))
    };

    let step1_class = if step >= 1 { "bg-blue-600 text-white" } else { "bg-gray-300 text-white" };
    let step2_class = if step >= 2 { "bg-blue-600 text-white" } else { "bg-gray-300 text-white" };
    let step3_class = if step >= 3 { "bg-blue-600 text-white" } else { "bg-gray-300 text-white" };
    let line1_class = if step >= 2 { "bg-blue-600" } else { "bg-gray-300" };
    let line2_class = if step >= 3 { "bg-blue-600" } else { "bg-gray-300" };

    let body = match step {
        1 => format!(r#"
            <h2 class="text-xl font-bold text-gray-800 mb-1">Create Admin Account</h2>
            <p class="text-sm text-gray-500 mb-6">Set up the administrator account for your forum</p>
            {error_html}
            <form method="POST" action="/setup" class="space-y-4">
                <input type="hidden" name="step" value="1">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Username</label>
                    <input type="text" name="username" required minlength="2" maxlength="20"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                        placeholder="admin">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Email</label>
                    <input type="email" name="email" required
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                        placeholder="admin@example.com">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Password</label>
                    <input type="password" name="password" required minlength="6"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                        placeholder="At least 6 characters">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Confirm Password</label>
                    <input type="password" name="password_confirm" required
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                        placeholder="Repeat password">
                </div>
                <button type="submit"
                    class="w-full bg-blue-600 text-white py-2.5 rounded-lg hover:bg-blue-700 transition font-medium">
                    Next &rarr;
                </button>
            </form>"#, error_html = error_html),
        2 => format!(r#"
            <h2 class="text-xl font-bold text-gray-800 mb-1">Site Settings</h2>
            <p class="text-sm text-gray-500 mb-6">Configure your forum's basic information</p>
            {error_html}
            <form method="POST" action="/setup" class="space-y-4">
                <input type="hidden" name="step" value="2">
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Site Name</label>
                    <input type="text" name="site_name" required
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                        value="RustForum">
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Site Description</label>
                    <textarea name="site_description" rows="2"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                        placeholder="A modern forum system built with Rust">A modern forum system built with Rust + Axum + SQLite</textarea>
                </div>
                <div>
                    <label class="block text-sm font-medium text-gray-700 mb-1">Keywords</label>
                    <input type="text" name="site_keywords"
                        class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none"
                        value="forum,rust,axum,sqlite"
                        placeholder="Comma-separated keywords">
                </div>
                <div class="flex gap-3">
                    <a href="/setup" class="flex-1 text-center py-2.5 rounded-lg border border-gray-300 text-gray-600 hover:bg-gray-50 transition font-medium">
                        &larr; Back
                    </a>
                    <button type="submit"
                        class="flex-1 bg-blue-600 text-white py-2.5 rounded-lg hover:bg-blue-700 transition font-medium">
                        Complete Setup
                    </button>
                </div>
            </form>"#, error_html = error_html),
        3 => r#"
            <div class="text-center">
                <div class="w-16 h-16 bg-green-100 text-green-600 rounded-full flex items-center justify-center mx-auto mb-4">
                    <i class="fa fa-check text-3xl"></i>
                </div>
                <h2 class="text-xl font-bold text-gray-800 mb-2">Setup Complete!</h2>
                <p class="text-sm text-gray-500 mb-6">Your forum is ready. Click below to enter.</p>
                <a href="/"
                    class="inline-block bg-blue-600 text-white px-8 py-2.5 rounded-lg hover:bg-blue-700 transition font-medium">
                    Enter Forum &rarr;
                </a>
            </div>"#.to_string(),
        _ => String::new(),
    };

    format!(r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Setup - RustForum</title>
    <script src="/static/css/tailwind.js"></script>
    <link rel="stylesheet" href="/static/css/font-awesome.min.css">
</head>
<body class="bg-gray-50 min-h-screen flex items-center justify-center p-4">
    <div class="w-full max-w-md">
        <div class="text-center mb-8">
            <h1 class="text-2xl font-bold text-gray-900">RustForum Setup</h1>
            <p class="text-sm text-gray-500 mt-1">Installation Wizard</p>
        </div>

        <!-- Progress bar -->
        <div class="flex items-center justify-center mb-8">
            <div class="flex items-center">
                <div class="{step1_class} w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold">1</div>
                <div class="{line1_class} w-16 h-1 mx-1"></div>
                <div class="{step2_class} w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold">2</div>
                <div class="{line2_class} w-16 h-1 mx-1"></div>
                <div class="{step3_class} w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold">3</div>
            </div>
        </div>

        <div class="bg-white rounded-xl border border-gray-100 shadow-sm p-6">
            {body}
        </div>

        <p class="text-center text-xs text-gray-400 mt-6">Powered by RustForum</p>
    </div>
</body>
</html>"##,
        step1_class = step1_class,
        step2_class = step2_class,
        step3_class = step3_class,
        line1_class = line1_class,
        line2_class = line2_class,
        body = body,
    )
}
