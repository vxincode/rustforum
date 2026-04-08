// Frontend API Interaction
// No third-party dependencies, pure vanilla JS

// =====================================================================
// Multiavatar initialization
// =====================================================================
function initMultiavatar() {
  if (typeof multiavatar === 'undefined') return;
  document.querySelectorAll('[data-multiavatar]').forEach(function(el) {
    if (el.querySelector('svg')) return; // already initialized
    var seed = el.getAttribute('data-multiavatar');
    el.innerHTML = multiavatar(seed);
  });
}

// =====================================================================
// Toast notification
// =====================================================================
function showToast(msg, duration = 3000) {
  const toast = document.getElementById('toast');
  const toastMsg = document.getElementById('toastMsg');
  if (!toast || !toastMsg) return;
  toastMsg.textContent = msg;
  toast.classList.remove('hidden');
  toast.style.animation = 'fadeIn 0.3s ease';
  clearTimeout(window._toastTimer);
  window._toastTimer = setTimeout(() => toast.classList.add('hidden'), duration);
}

// =====================================================================
// Tab switching (Latest / Hot) on index page
// =====================================================================
window._currentTab = 'latest';

function switchTab(tab) {
  window._currentTab = tab;
  const tabs = {
    latest: document.getElementById('tabLatest'),
    essence: document.getElementById('tabEssence'),
    hot: document.getElementById('tabHot'),
  };
  if (!tabs.latest || !tabs.hot) return;

  Object.values(tabs).forEach(b => {
    if (!b) return;
    b.classList.remove('font-medium', 'border-b-2', 'border-black');
    b.classList.add('text-gray-500');
  });
  const active = tabs[tab];
  if (active) {
    active.classList.remove('text-gray-500');
    active.classList.add('font-medium', 'border-b-2', 'border-black');
  }

  fetch(`/api/threads?tab=${tab}`, { credentials: 'same-origin' })
    .then(r => r.json())
    .then(data => renderThreadList(data))
    .catch(() => showToast('加载失败'));
}

function renderThreadList(data) {
  const container = document.getElementById('threadList');
  if (!container) return;

  if (!data.threads || data.threads.length === 0) {
    container.innerHTML = '<div class="px-5 py-12 text-center text-gray-400 text-sm">暂无帖子</div>';
    return;
  }

  container.innerHTML = data.threads.map(t => {
    const username = t.username || '未知';
    const time = (t.created_at || '').substring(0, 10);
    const avatarHtml = t.avatar
      ? `<img src="/static/avatars/${escapeHtml(t.avatar)}" class="w-9 h-9 rounded-full object-cover flex-shrink-0">`
      : `<span class="w-9 h-9 rounded-full overflow-hidden flex-shrink-0" data-multiavatar="${escapeHtml(username)}"></span>`;
    let badges = '';
    if (t.is_top === 1) badges += '<span class="text-xs bg-red-100 text-red-600 px-1.5 py-0.5 rounded">置顶</span> ';
    if (t.is_essence === 1) badges += '<span class="text-xs bg-orange-100 text-orange-600 px-1.5 py-0.5 rounded">精华</span> ';
    if (t.is_closed === 1) badges += '<span class="text-xs bg-gray-200 text-gray-500 px-1.5 py-0.5 rounded">已关闭</span> ';
    return `<div class="item-hover px-5 py-4 border-b border-gray-100 cursor-pointer" onclick="location.href='/thread/${t.id}'">
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-4">
          ${avatarHtml}
          <div>
            <h3 class="font-medium text-base">${badges}${escapeHtml(t.title)}</h3>
            <div class="flex items-center gap-3 mt-1 text-xs text-gray-500">
              <span>${escapeHtml(username)}</span>
            </div>
          </div>
        </div>
        <div class="flex items-center gap-4 text-xs text-gray-500 flex-shrink-0">
          <span><i class="fa fa-eye"></i> ${t.view_count}</span>
          <span><i class="fa fa-comment"></i> ${t.reply_count}</span>
          <span class="text-gray-400">${time}</span>
        </div>
      </div>
    </div>`;
  }).join('');

  initMultiavatar();
  renderPagination(data.page, data.total_pages);
}

function renderPagination(current, total) {
  const el = document.getElementById('pagination');
  if (!el || total <= 1) { if (el) el.innerHTML = ''; return; }

  let html = '<div class="flex gap-1">';
  if (current > 1) {
    html += `<button onclick="loadPage(${current - 1})" class="w-9 h-9 flex items-center justify-center rounded hover:bg-gray-100 text-gray-600"><i class="fa fa-angle-left"></i></button>`;
  }
  for (let i = Math.max(1, current - 3); i <= Math.min(total, current + 3); i++) {
    if (i === current) {
      html += `<span class="w-9 h-9 flex items-center justify-center rounded bg-black text-white">${i}</span>`;
    } else {
      html += `<button onclick="loadPage(${i})" class="w-9 h-9 flex items-center justify-center rounded hover:bg-gray-100">${i}</button>`;
    }
  }
  if (current < total) {
    html += `<button onclick="loadPage(${current + 1})" class="w-9 h-9 flex items-center justify-center rounded hover:bg-gray-100 text-gray-600"><i class="fa fa-angle-right"></i></button>`;
  }
  html += '</div>';
  el.innerHTML = html;
}

function loadPage(page) {
  const tab = window._currentTab || 'latest';
  fetch(`/api/threads?tab=${tab}&page=${page}`, { credentials: 'same-origin' })
    .then(r => r.json())
    .then(data => renderThreadList(data))
    .catch(() => showToast('加载失败'));
}

// =====================================================================
// Search
// =====================================================================
function doSearch() {
  const input = document.getElementById('searchInput');
  if (!input) return;
  const q = input.value.trim();
  if (!q) return;

  fetch(`/api/search?q=${encodeURIComponent(q)}`, { credentials: 'same-origin' })
    .then(r => r.json())
    .then(data => {
      if (data.results && data.results.length > 0) {
        const container = document.getElementById('threadList');
        if (container) {
          container.innerHTML = data.results.map(t => {
            const username = t.username || '未知';
            const avatarHtml = t.avatar
              ? `<img src="/static/avatars/${escapeHtml(t.avatar)}" class="w-9 h-9 rounded-full object-cover flex-shrink-0">`
              : `<span class="w-9 h-9 rounded-full overflow-hidden flex-shrink-0" data-multiavatar="${escapeHtml(username)}"></span>`;
            let badges = '';
            if (t.is_top === 1) badges += '<span class="text-xs bg-red-100 text-red-600 px-1.5 py-0.5 rounded">置顶</span> ';
            if (t.is_essence === 1) badges += '<span class="text-xs bg-orange-100 text-orange-600 px-1.5 py-0.5 rounded">精华</span> ';
            return `<div class="item-hover px-5 py-4 border-b border-gray-100 cursor-pointer" onclick="location.href='/thread/${t.id}'">
              <div class="flex items-center justify-between">
                <div class="flex items-center gap-4">
                  ${avatarHtml}
                  <div><h3 class="font-medium text-base">${badges}${escapeHtml(t.title)}</h3>
                  <div class="flex items-center gap-3 mt-1 text-xs text-gray-500"><span>${escapeHtml(username)}</span></div></div>
                </div>
                <div class="flex items-center gap-4 text-xs text-gray-500 flex-shrink-0">
                  <span><i class="fa fa-eye"></i> ${t.view_count}</span>
                  <span><i class="fa fa-comment"></i> ${t.reply_count}</span>
                </div>
              </div>
            </div>`;
          }).join('');
          initMultiavatar();
        }
        showToast(`找到 ${data.results.length} 个结果`);
      } else {
        showToast('未找到相关帖子');
      }
    })
    .catch(() => showToast('搜索失败'));
}

document.addEventListener('keydown', function(e) {
  if (e.key === 'Enter' && e.target.id === 'searchInput') {
    doSearch();
  }
});

// =====================================================================
// Load sidebar data (categories + stats + leaderboard + new users + links)
// =====================================================================
function loadSidebar() {
  var opts = { credentials: 'same-origin' };
  var icons = ['fa-laptop', 'fa-file-text', 'fa-question-circle', 'fa-book', 'fa-code', 'fa-comments'];

  Promise.all([
    fetch('/api/forums', opts).then(function(r) { return r.json(); }).catch(function() { return null; }),
    fetch('/api/users/recent', opts).then(function(r) { return r.json(); }).catch(function() { return null; }),
    fetch('/api/leaderboard', opts).then(function(r) { return r.json(); }).catch(function() { return null; }),
    fetch('/api/stats', opts).then(function(r) { return r.json(); }).catch(function() { return null; }),
    fetch('/api/links', opts).then(function(r) { return r.json(); }).catch(function() { return null; }),
  ]).then(function(results) {
    var forums = results[0];
    var users = results[1];
    var lb = results[2];
    var stats = results[3];
    var links = results[4];

    // 分类
    if (forums && forums.forums) {
      var el = document.getElementById('categoryList');
      if (el) {
        el.innerHTML = forums.forums.map(function(f, i) {
          var icon = icons[i % icons.length];
          return '<a href="/forum/' + f.id + '" class="flex justify-between p-2 rounded hover:bg-gray-100 transition-colors">' +
            '<span class="flex items-center gap-1"><i class="fa ' + icon + ' text-gray-500"></i> ' + escapeHtml(f.name) + '</span>' +
            '<span class="bg-gray-200 px-1.5 rounded text-xs">' + f.thread_count + '</span></a>';
        }).join('');
      }
    }

    // 新会员
    if (users && users.users) {
      var el2 = document.getElementById('newUsers');
      if (el2) {
        if (users.users.length === 0) {
          el2.innerHTML = '<div class="text-gray-400 text-xs p-1.5">暂无新会员</div>';
        } else {
          el2.innerHTML = users.users.map(function(u) {
            var avatarHtml = u.avatar
              ? '<img src="/static/avatars/' + escapeHtml(u.avatar) + '" class="w-7 h-7 rounded-full object-cover flex-shrink-0">'
              : '<span class="w-7 h-7 rounded-full overflow-hidden flex-shrink-0" data-multiavatar="' + escapeHtml(u.username) + '"></span>';
            return '<a href="/user/' + u.id + '" class="flex items-center gap-2 p-1.5 rounded hover:bg-gray-100 transition-colors">' +
              avatarHtml + '<span class="truncate">' + escapeHtml(u.username) + '</span>' +
              '<span class="text-gray-400 ml-auto">' + escapeHtml(u.rank_title) + '</span></a>';
          }).join('');
        }
      }
    }

    // 积分排行
    if (lb && lb.users) {
      var el3 = document.getElementById('leaderboard');
      if (el3) {
        if (lb.users.length === 0) {
          el3.innerHTML = '<div class="text-gray-400 text-xs p-1.5">暂无数据</div>';
        } else {
          el3.innerHTML = lb.users.map(function(u, i) {
            var avatarHtml = u.avatar
              ? '<img src="/static/avatars/' + escapeHtml(u.avatar) + '" class="w-6 h-6 rounded-full object-cover flex-shrink-0">'
              : '<span class="w-6 h-6 rounded-full overflow-hidden flex-shrink-0" data-multiavatar="' + escapeHtml(u.username) + '"></span>';
            var medal = i === 0 ? '<i class="fa fa-trophy text-yellow-500"></i>' : (i === 1 ? '<i class="fa fa-trophy text-gray-400"></i>' : (i === 2 ? '<i class="fa fa-trophy text-amber-700"></i>' : '<span class="text-gray-400 w-4 text-center">' + (i+1) + '</span>'));
            return '<a href="/user/' + u.id + '" class="flex items-center gap-2 p-1.5 rounded hover:bg-gray-100 transition-colors">' +
              medal + avatarHtml + '<span class="truncate flex-1">' + escapeHtml(u.username) + '</span>' +
              '<span class="text-gray-500">' + u.credits + '</span></a>';
          }).join('');
        }
      }
    }

    // 社区统计
    if (stats) {
      var el4 = document.getElementById('siteStats');
      if (el4) {
        el4.innerHTML =
          '<div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">' + (stats.posts || 0) + '</p><p class="text-gray-500 text-xs">帖子</p></div>' +
          '<div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">' + (stats.replies || 0) + '</p><p class="text-gray-500 text-xs">回复</p></div>' +
          '<div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">' + (stats.users || 0) + '</p><p class="text-gray-500 text-xs">会员</p></div>' +
          '<div class="p-2 bg-gray-50 rounded border border-gray-100"><p class="font-semibold">' + (stats.today_checkins || 0) + '</p><p class="text-gray-500 text-xs">今日签到</p></div>';
      }
    }

    // 友情链接
    if (links && links.links) {
      var el5 = document.getElementById('friendlyLinks');
      if (el5) {
        if (links.links.length === 0) {
          el5.innerHTML = '<div class="text-gray-400 text-xs p-1.5">暂无链接</div>';
        } else {
          el5.innerHTML = links.links.map(function(l) {
            return '<a href="' + escapeHtml(l.url) + '" target="_blank" rel="noopener" class="block p-1.5 hover:bg-gray-100 rounded transition-colors text-gray-600 hover:text-black truncate">' +
              '<i class="fa fa-link text-gray-400 mr-1"></i>' + escapeHtml(l.name) + '</a>';
          }).join('');
        }
      }
    }

    // 所有数据渲染完后统一初始化头像
    initMultiavatar();
  });
}

// =====================================================================
// Checkin
// =====================================================================
function doCheckin() {
  const btn = document.getElementById('checkinBtn');
  const btnText = document.getElementById('checkinBtnText');
  if (!btn) return;
  btn.disabled = true;
  btnText.textContent = '签到中...';

  fetch('/api/checkin', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
  })
  .then(r => r.json())
  .then(res => {
    if (res.ok) {
      btnText.textContent = '已签到';
      btn.classList.remove('bg-gray-100', 'text-black', 'hover:bg-gray-200');
      btn.classList.add('bg-green-100', 'text-green-700');
      const info = document.getElementById('checkinInfo');
      if (info) {
        info.classList.remove('hidden');
        info.textContent = `连续${res.streak}天签到，+${res.credits_gained}积分`;
      }
      showToast(`签到成功！连续${res.streak}天，获得${res.credits_gained}积分`);
    } else {
      btn.disabled = false;
      btnText.textContent = '签到';
      showToast(res.error || '签到失败');
    }
  })
  .catch(() => {
    btn.disabled = false;
    btnText.textContent = '签到';
    showToast('网络错误');
  });
}

function loadCheckinStatus() {
  fetch('/api/checkin/status', { credentials: 'same-origin' })
    .then(r => r.json())
    .then(res => {
      if (!res.ok) return;
      const btn = document.getElementById('checkinBtn');
      const btnText = document.getElementById('checkinBtnText');
      if (!btn || !btnText) return;
      if (res.checked_in) {
        btn.disabled = true;
        btnText.textContent = '已签到';
        btn.classList.remove('bg-gray-100', 'text-black', 'hover:bg-gray-200');
        btn.classList.add('bg-green-100', 'text-green-700');
      }
      const info = document.getElementById('checkinInfo');
      if (info && res.streak > 0) {
        info.classList.remove('hidden');
        info.textContent = `已连续签到${res.streak}天`;
      }
    })
    .catch(() => {});
}

// =====================================================================
// Auth: Login — server sets HttpOnly cookie via Set-Cookie header
// =====================================================================
function submitLogin(e) {
  e.preventDefault();
  const form = e.target;
  const btn = form.querySelector('button[type="submit"]');
  const origText = btn.textContent;
  btn.disabled = true;
  btn.textContent = '登录中...';

  fetch('/api/auth/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify({
      username: form.username.value,
      password: form.password.value,
    }),
  })
  .then(r => r.json())
  .then(res => {
    if (res.ok) {
      showToast('登录成功，正在跳转...');
      setTimeout(() => location.href = '/', 800);
    } else {
      btn.disabled = false;
      btn.textContent = origText;
      showToast(res.error || '登录失败');
    }
  })
  .catch(() => {
    btn.disabled = false;
    btn.textContent = origText;
    showToast('网络错误');
  });
}

// =====================================================================
// Auth: Register
// =====================================================================
function submitRegister(e) {
  e.preventDefault();
  const form = e.target;
  const btn = form.querySelector('button[type="submit"]');
  const origText = btn.textContent;
  btn.disabled = true;
  btn.textContent = '注册中...';

  fetch('/api/auth/register', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify({
      username: form.username.value,
      email: form.email.value,
      password: form.password.value,
      password_confirm: form.password_confirm.value,
    }),
  })
  .then(r => r.json())
  .then(res => {
    if (res.ok) {
      showToast('注册成功，请登录');
      setTimeout(() => location.href = '/auth/login', 800);
    } else {
      btn.disabled = false;
      btn.textContent = origText;
      showToast(res.error || '注册失败');
    }
  })
  .catch(() => {
    btn.disabled = false;
    btn.textContent = origText;
    showToast('网络错误');
  });
}

// =====================================================================
// Auth: Logout
// =====================================================================
function doLogout() {
  fetch('/api/auth/logout', { credentials: 'same-origin' })
    .then(() => {
      showToast('已退出登录');
      setTimeout(() => location.href = '/', 500);
    })
    .catch(() => {
      location.href = '/';
    });
}

// =====================================================================
// New thread via API
// =====================================================================
function submitThread(e) {
  e.preventDefault();
  const title = document.getElementById('threadTitle').value.trim();
  const content = document.getElementById('threadContent').value.trim();
  const forumSelect = document.getElementById('forumSelect');
  const forumId = forumSelect ? forumSelect.value : '1';
  const btn = e.target.querySelector('button[type="submit"]');
  const origText = btn.innerHTML;
  btn.disabled = true;
  btn.innerHTML = '<i class="fa fa-spinner fa-spin mr-1"></i>发布中...';

  fetch(`/api/thread/${forumId}/new`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify({ title, content }),
  })
  .then(r => {
    if (r.status === 302 || r.redirected) {
      showToast('请先登录');
      setTimeout(() => location.href = '/auth/login', 1000);
      return null;
    }
    return r.json();
  })
  .then(res => {
    if (!res) return;
    if (res.ok) {
      showToast('发布成功！');
      setTimeout(() => location.href = `/thread/${res.thread_id}`, 800);
    } else {
      btn.disabled = false;
      btn.innerHTML = origText;
      showToast(res.error || '发布失败');
    }
  })
  .catch(() => {
    btn.disabled = false;
    btn.innerHTML = origText;
    showToast('网络错误');
  });
}

// =====================================================================
// Reply via API
// =====================================================================
function submitReply(e, threadId) {
  e.preventDefault();
  const contentEl = document.getElementById('replyContent');
  const content = contentEl.value.trim();
  if (!content) {
    showToast('请输入回复内容');
    return;
  }
  const btn = e.target.querySelector('button[type="submit"]');
  const origText = btn.innerHTML;
  btn.disabled = true;
  btn.innerHTML = '<i class="fa fa-spinner fa-spin mr-1"></i>提交中...';

  fetch(`/api/thread/${threadId}/reply`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    credentials: 'same-origin',
    body: JSON.stringify({ content }),
  })
  .then(r => {
    if (r.status === 302 || r.redirected) {
      showToast('请先登录');
      setTimeout(() => location.href = '/auth/login', 1000);
      return null;
    }
    return r.json();
  })
  .then(res => {
    if (!res) return;
    if (res.ok) {
      showToast('回复成功！');
      setTimeout(() => location.reload(), 800);
    } else {
      btn.disabled = false;
      btn.innerHTML = origText;
      showToast(res.error || '回复失败');
    }
  })
  .catch(() => {
    btn.disabled = false;
    btn.innerHTML = origText;
    showToast('网络错误');
  });
}

// =====================================================================
// HTML escape helper
// =====================================================================
function escapeHtml(str) {
  if (!str) return '';
  return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
}

// =====================================================================
// Generic POST action helper
// =====================================================================
function postAction(url) {
  fetch(url, { method: 'POST', credentials: 'same-origin' })
    .then(r => {
      if (r.redirected) {
        location.href = r.url;
      } else {
        location.reload();
      }
    })
    .catch(() => showToast('操作失败'));
}

// =====================================================================
// Profile edit
// =====================================================================
function submitProfileEdit(e) {
  e.preventDefault();
  const email = document.getElementById('editEmail').value.trim();
  const signature = document.getElementById('editSignature').value.trim();
  const customTitle = document.getElementById('editCustomTitle') ? document.getElementById('editCustomTitle').value.trim() : '';
  const epithet = document.getElementById('editEpithet') ? document.getElementById('editEpithet').value.trim() : '';
  const epithetColor = document.getElementById('editEpithetColor') ? document.getElementById('editEpithetColor').value : '';
  if (!email) { showToast('邮箱不能为空'); return; }

  const btn = e.target.querySelector('button[type="submit"]');
  const origText = btn.innerHTML;
  btn.disabled = true;
  btn.innerHTML = '<i class="fa fa-spinner fa-spin mr-1"></i>保存中...';

  const params = new URLSearchParams();
  params.set('email', email);
  params.set('signature', signature);
  params.set('custom_title', customTitle);
  params.set('epithet', epithet);
  params.set('epithet_color', epithetColor);

  fetch('/profile/edit', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    credentials: 'same-origin',
    body: params.toString(),
  })
  .then(r => {
    if (r.redirected) {
      showToast('保存成功');
      setTimeout(() => location.href = r.url, 800);
    } else {
      return r.text().then(() => { showToast('保存失败'); });
    }
  })
  .catch(() => { showToast('网络错误'); })
  .finally(() => { btn.disabled = false; btn.innerHTML = origText; });
}

// =====================================================================
// Change password
// =====================================================================
function submitChangePassword(e) {
  e.preventDefault();
  const oldPassword = document.getElementById('oldPassword').value;
  const newPassword = document.getElementById('newPassword').value;
  const confirmPassword = document.getElementById('confirmPassword').value;

  if (!oldPassword || !newPassword) { showToast('请填写密码'); return; }
  if (newPassword.length < 6) { showToast('新密码至少6位'); return; }
  if (newPassword !== confirmPassword) { showToast('两次密码不一致'); return; }

  const btn = e.target.querySelector('button[type="submit"]');
  const origText = btn.innerHTML;
  btn.disabled = true;
  btn.innerHTML = '<i class="fa fa-spinner fa-spin mr-1"></i>修改中...';

  fetch('/profile/password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    credentials: 'same-origin',
    body: 'old_password=' + encodeURIComponent(oldPassword) +
          '&new_password=' + encodeURIComponent(newPassword) +
          '&confirm_password=' + encodeURIComponent(confirmPassword),
  })
  .then(r => {
    if (r.redirected) {
      showToast('密码修改成功');
      setTimeout(() => location.href = r.url, 800);
    } else {
      return r.text().then(() => { showToast('修改失败，请检查旧密码'); });
    }
  })
  .catch(() => { showToast('网络错误'); })
  .finally(() => { btn.disabled = false; btn.innerHTML = origText; });
}

// =====================================================================
// Delete thread confirmation
// =====================================================================
function confirmDeleteThread(threadId) {
  if (!confirm('确定要删除这个主题吗？所有回复也将被删除，此操作不可恢复。')) return;
  postAction('/thread/' + threadId + '/delete');
}

// =====================================================================
// Delete post confirmation
// =====================================================================
function confirmDeletePost(postId) {
  if (!confirm('确定要删除这条回复吗？此操作不可恢复。')) return;
  postAction('/post/' + postId + '/delete');
}

// =====================================================================
// Admin: toggle sticky
// =====================================================================
function adminToggleSticky(threadId) {
  postAction('/admin/thread/' + threadId + '/sticky');
}

// =====================================================================
// Admin: toggle essence
// =====================================================================
function adminToggleEssence(threadId) {
  postAction('/admin/thread/' + threadId + '/essence');
}

// =====================================================================
// Admin: toggle close
// =====================================================================
function adminToggleClose(threadId) {
  postAction('/admin/thread/' + threadId + '/close');
}

// =====================================================================
// Admin: delete thread
// =====================================================================
function adminDeleteThread(threadId) {
  if (!confirm('确定要删除这个主题吗？所有回复也将被删除。')) return;
  postAction('/admin/thread/' + threadId + '/delete');
}

// =====================================================================
// Admin: delete post
// =====================================================================
function adminDeletePost(postId) {
  if (!confirm('确定要删除这条回复吗？')) return;
  postAction('/admin/post/' + postId + '/delete');
}

// =====================================================================
// Avatar upload
// =====================================================================
function submitAvatar(e) {
  e.preventDefault();
  var input = document.getElementById('avatarInput');
  if (!input || !input.files || !input.files[0]) {
    showToast('请选择文件');
    return;
  }
  var file = input.files[0];
  if (file.size > 512 * 1024) {
    showToast('文件不能超过512KB');
    return;
  }
  var allowedTypes = ['image/jpeg', 'image/png', 'image/gif', 'image/webp'];
  if (allowedTypes.indexOf(file.type) === -1) {
    showToast('只支持 JPG/PNG/GIF/WebP 格式');
    return;
  }

  var formData = new FormData();
  formData.append('avatar', file);

  fetch('/profile/avatar', {
    method: 'POST',
    credentials: 'same-origin',
    body: formData,
  })
  .then(function(r) {
    if (r.redirected) {
      showToast('头像上传成功');
      setTimeout(function() { location.href = r.url; }, 800);
    } else {
      return r.text().then(function() { showToast('上传失败'); });
    }
  })
  .catch(function() { showToast('网络错误'); });
}

// =====================================================================
// Load unread message count
// =====================================================================
function loadUnreadCount() {
  fetch('/api/messages/unread', { credentials: 'same-origin' })
    .then(function(r) { return r.json(); })
    .then(function(data) {
      var badge = document.getElementById('msgBadge');
      if (badge && data.count > 0) {
        badge.textContent = data.count > 9 ? '9+' : data.count;
        badge.classList.remove('hidden');
      }
    })
    .catch(function() {});
}

// =====================================================================
// Init on page load
// =====================================================================
document.addEventListener('DOMContentLoaded', function() {
  if (document.getElementById('categoryList') || document.getElementById('siteStats')) {
    loadSidebar();
  }
  // Load checkin status if checkin button exists (logged in)
  if (document.getElementById('checkinBtn')) {
    loadCheckinStatus();
  }
  // Load unread message count if logged in (sidebar exists)
  if (document.getElementById('msgBadge')) {
    loadUnreadCount();
  }
  // Load notification badge
  if (document.getElementById('notifBadge')) {
    loadNotifBadge();
  }
  // Auto-scroll chat to bottom
  var chatBox = document.getElementById('chatBox');
  if (chatBox) {
    chatBox.scrollTop = chatBox.scrollHeight;
  }
  // Initialize multiavatar placeholders
  initMultiavatar();
});

// =====================================================================
// User hover card
// =====================================================================
(function() {
  var cardEl = document.getElementById('userCardPopup');
  if (!cardEl) return;

  var cache = {};
  var showTimer = null;
  var hideTimer = null;
  var currentTarget = null;

  function showCard(target) {
    var uid = target.getAttribute('data-user-card');
    if (!uid) return;

    clearTimeout(hideTimer);
    var rect = target.getBoundingClientRect();
    var cardWidth = 240;
    var left = rect.left + rect.width / 2 - cardWidth / 2;
    var top = rect.bottom + 8;

    // Keep within viewport
    if (left < 8) left = 8;
    if (left + cardWidth > window.innerWidth - 8) left = window.innerWidth - cardWidth - 8;
    if (top + 200 > window.innerHeight) top = rect.top - 208;

    cardEl.style.left = left + 'px';
    cardEl.style.top = top + 'px';
    cardEl.style.width = cardWidth + 'px';

    if (cache[uid]) {
      cardEl.innerHTML = cache[uid];
      cardEl.classList.add('show');
      return;
    }

    // Show loading
    cardEl.innerHTML = '<div style="text-align:center;color:#999;font-size:12px;padding:12px 0">加载中...</div>';
    cardEl.classList.add('show');

    fetch('/api/user/' + uid + '/card', { credentials: 'same-origin' })
      .then(function(r) { return r.json(); })
      .then(function(data) {
        if (!data.ok) {
          cardEl.classList.remove('show');
          return;
        }
        var u = data.user;
        var initial = (u.username || 'U')[0].toUpperCase();
        var avatarHtml = u.avatar
          ? '<img src="/static/avatars/' + escapeHtml(u.avatar) + '" style="width:48px;height:48px;border-radius:50%;object-fit:cover">'
          : '<span style="width:48px;height:48px;border-radius:50%;overflow:hidden;display:inline-flex" data-multiavatar="' + escapeHtml(u.username) + '"></span>';
        var sigHtml = u.signature ? '<div style="font-size:12px;color:#999;margin-top:8px;padding-top:8px;border-top:1px solid #f0f0f0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + escapeHtml(u.signature) + '</div>' : '';
        var html = '<div style="display:flex;align-items:center;gap:12px">'
          + '<a href="/user/' + u.id + '" style="flex-shrink:0">' + avatarHtml + '</a>'
          + '<div style="min-width:0">'
          + '<div style="font-weight:600;font-size:14px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">' + escapeHtml(u.username) + '</div>'
          + '<div style="font-size:12px;color:#999">' + escapeHtml(u.group_name) + '</div>'
          + '</div></div>'
          + '<div style="display:grid;grid-template-columns:1fr 1fr 1fr;gap:8px;margin-top:12px;text-align:center;font-size:12px">'
          + '<div style="background:#f9f9f9;border-radius:8px;padding:6px 0"><div style="font-weight:600">' + u.thread_count + '</div><div style="color:#999">主题</div></div>'
          + '<div style="background:#f9f9f9;border-radius:8px;padding:6px 0"><div style="font-weight:600">' + u.post_count + '</div><div style="color:#999">帖子</div></div>'
          + '<div style="background:#f9f9f9;border-radius:8px;padding:6px 0"><div style="font-weight:600">' + u.credits + '</div><div style="color:#999">积分</div></div>'
          + '</div>'
          + '<div style="font-size:12px;color:#999;margin-top:8px"><i class="fa fa-calendar-o" style="margin-right:4px"></i>' + u.join_date + ' 加入</div>'
          + sigHtml;
        cache[uid] = html;
        cardEl.innerHTML = html;
        initMultiavatar();
      })
      .catch(function() { cardEl.classList.remove('show'); });
  }

  function hideCard() {
    clearTimeout(showTimer);
    hideTimer = setTimeout(function() {
      cardEl.classList.remove('show');
      currentTarget = null;
    }, 200);
  }

  // Use mouseover/mouseout with relatedTarget check to avoid child-element flickering
  document.addEventListener('mouseover', function(e) {
    var target = e.target.closest('[data-user-card]');
    if (!target) return;
    // Check if we're entering from outside the target
    var related = e.relatedTarget ? e.relatedTarget.closest('[data-user-card]') : null;
    if (related === target) return; // still inside same target, ignore
    clearTimeout(hideTimer);
    clearTimeout(showTimer);
    currentTarget = target;
    showTimer = setTimeout(function() { showCard(target); }, 400);
  });

  document.addEventListener('mouseout', function(e) {
    var target = e.target.closest('[data-user-card]');
    if (!target) return;
    var related = e.relatedTarget ? e.relatedTarget.closest('[data-user-card]') : null;
    if (related === target) return; // still inside same target, ignore
    clearTimeout(showTimer);
    hideCard();
  });

  // Keep card visible when mouse moves onto the card itself
  cardEl.addEventListener('mouseenter', function() {
    clearTimeout(hideTimer);
  });
  cardEl.addEventListener('mouseleave', function() {
    hideCard();
  });
})();

// =====================================================================
// Editor: Insert formatting into textarea
// =====================================================================
function _getEditor(id) {
  return document.getElementById(id || 'replyContent');
}

function insertFormat(before, after) { insertFormatTo('replyContent', before, after); }
function insertBlock(before, after) { insertBlockTo('replyContent', before, after); }
function insertPrefix(prefix) { insertPrefixTo('replyContent', prefix); }

function insertFormatTo(id, before, after) {
  var el = _getEditor(id);
  if (!el) return;
  var start = el.selectionStart;
  var end = el.selectionEnd;
  var text = el.value;
  var selected = text.substring(start, end) || '文本';
  el.value = text.substring(0, start) + before + selected + after + text.substring(end);
  el.focus();
  el.selectionStart = start + before.length;
  el.selectionEnd = start + before.length + selected.length;
}

function insertBlockTo(id, before, after) {
  var el = _getEditor(id);
  if (!el) return;
  var start = el.selectionStart;
  var end = el.selectionEnd;
  var text = el.value;
  var selected = text.substring(start, end) || '';
  el.value = text.substring(0, start) + before + selected + after + text.substring(end);
  el.focus();
  el.selectionStart = start + before.length;
  el.selectionEnd = start + before.length + selected.length;
}

function insertPrefixTo(id, prefix) {
  var el = _getEditor(id);
  if (!el) return;
  var start = el.selectionStart;
  var text = el.value;
  // Insert at beginning of current line
  var lineStart = text.lastIndexOf('\n', start - 1) + 1;
  el.value = text.substring(0, lineStart) + prefix + text.substring(lineStart);
  el.focus();
  el.selectionStart = el.selectionEnd = lineStart + prefix.length;
}

// =====================================================================
// Quote reply — scroll to bottom reply form and insert [quote]
// =====================================================================
function quoteReply(username, content) {
  var el = document.getElementById('replyContent');
  if (!el) {
    showToast('请先登录');
    return;
  }
  var section = document.getElementById('replySection');
  if (section) {
    section.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }
  var quote = '[quote=' + username + ']' + content + '[/quote]\n';
  var current = el.value;
  // Avoid duplicate quotes
  if (current.indexOf(quote.trim()) !== -1) {
    showToast('已引用该内容');
    return;
  }
  el.value = current + (current && !current.endsWith('\n') ? '\n' : '') + quote;
  el.focus();
  el.scrollTop = el.scrollHeight;
}

// =====================================================================
// Quick reply — mention user in reply form
// =====================================================================
function quickReply(username, floor) {
  var el = document.getElementById('replyContent');
  if (!el) {
    showToast('请先登录');
    return;
  }
  var section = document.getElementById('replySection');
  if (section) {
    section.scrollIntoView({ behavior: 'smooth', block: 'center' });
  }
  var mention = '@' + username + ' ';
  var current = el.value;
  el.value = current + (current && !current.endsWith('\n') ? '\n' : '') + mention;
  el.focus();
  el.scrollTop = el.scrollHeight;
}

// =====================================================================
// Emoji picker
// =====================================================================
var emojiList = [
  ['😊','smile'],['😂','joy'],['🤣','rofl'],['😍','heart_eyes'],['🤔','thinking'],
  ['😎','cool'],['😢','cry'],['😡','angry'],['👍','+1'],['👎','-1'],
  ['❤️','heart'],['🔥','fire'],['✨','sparkles'],['🎉','tada'],['💯','100'],
  ['🙏','pray'],['💪','muscle'],['👀','eyes'],['🤝','handshake'],['💡','bulb'],
  ['🚀','rocket'],['⭐','star'],['📌','pushpin'],['✅','white_check_mark'],
  ['❌','x'],['⚡','zap'],['🎯','dart'],['💬','speech_balloon'],['🔔','bell'],
  ['📖','book'],['💻','computer'],['🎨','art'],['🎵','music'],['🌍','globe'],
  ['☕','coffee'],['🍺','beer'],['🎂','birthday'],['🌸','cherry_blossom'],
  ['🍀','four_leaf_clover'],['🌈','rainbow'],['🌊','wave'],['🌙','moon'],
  ['☀️','sunny'],['⛈️','thunderstorm'],['❄️','snowflake'],['🎵','notes'],
  ['🎮','video_game'],['📱','phone'],['💡','idea'],['🔑','key'],['🎁','gift'],
  ['🏆','trophy'],['🎤','microphone'],['🎬','movie_camera'],['📷','camera'],
  ['✏️','pencil2'],['📝','memo'],['🔍','mag'],['💼','briefcase'],
];

function toggleEmojiPicker() {
  var picker = document.getElementById('emojiPicker');
  if (!picker) return;
  picker.classList.toggle('hidden');
  // Populate grid on first open
  var grid = document.getElementById('emojiGrid');
  if (grid && grid.children.length === 0) {
    grid.innerHTML = emojiList.map(function(e) {
      return '<button type="button" class="w-8 h-8 flex items-center justify-center rounded hover:bg-gray-100 transition-colors" onclick="insertEmojiTo(\'replyContent\',\'' + e[0] + '\',\'emojiPicker\')" title="' + e[1] + '">' + e[0] + '</button>';
    }).join('');
  }
}

function insertEmoji(emoji) {
  var el = document.getElementById('replyContent');
  if (!el) return;
  var start = el.selectionStart;
  var text = el.value;
  el.value = text.substring(0, start) + emoji + text.substring(el.selectionEnd);
  el.focus();
  el.selectionStart = el.selectionEnd = start + emoji.length;
  // Close picker
  var picker = document.getElementById('emojiPicker');
  if (picker) picker.classList.add('hidden');
}

// Close emoji picker when clicking outside
document.addEventListener('click', function(e) {
  var wrap = document.getElementById('emojiPickerWrap');
  var picker = document.getElementById('emojiPicker');
  if (!wrap || !picker) return;
  if (!wrap.contains(e.target)) {
    picker.classList.add('hidden');
  }
  // Also close new thread emoji picker
  var wrap2 = document.getElementById('newThreadEmojiWrap');
  var picker2 = document.getElementById('newThreadEmojiPicker');
  if (!wrap2 || !picker2) return;
  if (!wrap2.contains(e.target)) {
    picker2.classList.add('hidden');
  }
});

// =====================================================================
// Emoji picker for arbitrary textarea target
// =====================================================================
function toggleEmojiPickerFor(textareaId, gridId, pickerId) {
  var picker = document.getElementById(pickerId);
  if (!picker) return;
  picker.classList.toggle('hidden');
  var grid = document.getElementById(gridId);
  if (grid && grid.children.length === 0) {
    grid.innerHTML = emojiList.map(function(e) {
      return '<button type="button" class="w-8 h-8 flex items-center justify-center rounded hover:bg-gray-100 transition-colors" onclick="insertEmojiTo(\'' + textareaId + '\',\'' + e[0] + '\',\'' + pickerId + '\')" title="' + e[1] + '">' + e[0] + '</button>';
    }).join('');
  }
}

function insertEmojiTo(textareaId, emoji, pickerId) {
  var el = document.getElementById(textareaId);
  if (!el) return;
  var start = el.selectionStart;
  var text = el.value;
  el.value = text.substring(0, start) + emoji + text.substring(el.selectionEnd);
  el.focus();
  el.selectionStart = el.selectionEnd = start + emoji.length;
  var picker = document.getElementById(pickerId);
  if (picker) picker.classList.add('hidden');
}

// =====================================================================
// Notifications: Badge + Panel + Read
// =====================================================================
var _notifLoaded = false;
var _notifData = null;

function loadNotifBadge() {
  fetch('/api/notifications', { credentials: 'same-origin' })
    .then(function(r) { return r.json(); })
    .then(function(data) {
      _notifData = data;
      _notifLoaded = true;
      var total = data.total_unread || 0;
      var badge = document.getElementById('notifBadge');
      if (badge && total > 0) {
        badge.textContent = total > 9 ? '9+' : total;
        badge.classList.remove('hidden');
      }
      // Also update sidebar message badge
      var msgBadge = document.getElementById('msgBadge');
      if (msgBadge && data.unread_messages > 0) {
        msgBadge.textContent = data.unread_messages > 9 ? '9+' : data.unread_messages;
        msgBadge.classList.remove('hidden');
      }
    })
    .catch(function() {});
}

function toggleNotifPanel() {
  var panel = document.getElementById('notifPanel');
  if (!panel) return;
  if (panel.classList.contains('hidden')) {
    panel.classList.remove('hidden');
    if (_notifLoaded && _notifData) {
      renderNotifList(_notifData);
    } else {
      fetchNotifications();
    }
  } else {
    panel.classList.add('hidden');
  }
}

function fetchNotifications() {
  fetch('/api/notifications', { credentials: 'same-origin' })
    .then(function(r) { return r.json(); })
    .then(function(data) {
      _notifData = data;
      _notifLoaded = true;
      renderNotifList(data);
    })
    .catch(function() {});
}

function renderNotifList(data) {
  var list = document.getElementById('notifList');
  if (!list) return;

  var all = [];
  // Post interaction notifications (exclude 'message' type to avoid duplicate with summary line)
  (data.notifications || []).forEach(function(n) {
    if (n.type !== 'message') all.push(n);
  });
  // Unread messages summary
  if (data.unread_messages > 0) {
    all.push({ type: 'message_summary', content: data.unread_messages + ' 条未读私信', id: 'msg' });
  }

  if (all.length === 0) {
    list.innerHTML = '<div class="px-4 py-6 text-center text-gray-400 text-xs">暂无通知</div>';
    return;
  }

  list.innerHTML = all.map(function(n) {
    if (n.type === 'message_summary') {
      return '<a href="/messages" class="notif-item unread">' +
        '<div class="notif-dot"></div>' +
        '<div class="flex-1 min-w-0"><p class="text-sm truncate">📬 ' + escapeHtml(n.content) + '</p></div></a>';
    }
    var icon = n.type === 'reply'? '💬' : n.type === 'quote'? '🗨️' : n.type === 'system'? '⚙️' : '🔔';
    var url = n.thread_id ? '/thread/' + n.thread_id : '/messages';
    var time = (n.created_at || '').substring(5, 16);
    return '<a href="' + url + '" class="notif-item unread" onclick="markNotifRead(' + n.id + ', this)">' +
      '<div class="notif-dot"></div>' +
      '<div class="flex-1 min-w-0">' +
      '<p class="text-sm truncate">' + icon + ' ' + escapeHtml(n.content || '') + '</p>' +
      '<p class="text-xs text-gray-400 mt-1">' + escapeHtml(n.from_username || '') + ' · ' + time + '</p>' +
      '</div></a>';
  }).join('');
}

function markNotifRead(notifId, el) {
  fetch('/api/notifications/' + notifId + '/read', { method: 'POST', credentials: 'same-origin' })
    .then(function() {
      // 从 DOM 中移除该通知项
      if (el) { el.remove(); }
      // 检查是否还有剩余通知
      var remaining = document.querySelectorAll('.notif-item');
      if (remaining.length === 0) {
        var list = document.getElementById('notifList');
        if (list) list.innerHTML = '<div class="px-4 py-6 text-center text-gray-400 text-xs">暂无通知</div>';
      }
      // 更新 badge
      var badge = document.getElementById('notifBadge');
      if (badge && remaining.length === 0) {
        badge.classList.add('hidden');
      } else if (badge && remaining.length > 0) {
        badge.textContent = remaining.length > 9 ? '9+' : remaining.length;
        badge.classList.remove('hidden');
      }
    })
    .catch(function() {});
}

function markAllRead() {
  fetch('/api/notifications/read-all', { method: 'POST', credentials: 'same-origin' })
    .then(function() {
      // 清空所有通知
      var list = document.getElementById('notifList');
      if (list) list.innerHTML = '<div class="px-4 py-6 text-center text-gray-400 text-xs">暂无通知</div>';
      var badge = document.getElementById('notifBadge');
      if (badge) badge.classList.add('hidden');
    })
    .catch(function() {});
}

// Close notification panel when clicking outside
document.addEventListener('click', function(e) {
  var wrap = document.getElementById('notifBellWrap');
  var panel = document.getElementById('notifPanel');
  if (!wrap || !panel) return;
  if (!wrap.contains(e.target)) {
    panel.classList.add('hidden');
  }
});

// =====================================================================
// Inline editing: Edit post via bottom reply form
// =====================================================================
var _editMode = null; // { post_id, is_first, thread_id, original_content }

function editPost(postId, isFirst) {
  // Scroll to reply section
  var section = document.getElementById('replySection');
  if (!section) {
    showToast('请先登录');
    return;
  }
  section.scrollIntoView({ behavior: 'smooth', block: 'center' });

  // Fetch post content
  fetch('/api/post/' + postId, { credentials: 'same-origin' })
    .then(function(r) { return r.json(); })
    .then(function(data) {
      if (!data.ok) {
        showToast(data.error || '获取失败');
        return;
      }
      var post = data.post;
      var el = document.getElementById('replyContent');
      if (!el) return;
      // Save original for cancel
      _editMode = { post_id: post.id, is_first: post.is_first, thread_id: post.thread_id, original_content: el.value };
      el.value = post.content;
      // Update form UI to show edit mode
      updateReplyFormMode('edit', post);
    })
    .catch(function() { showToast('获取失败'); });
}

function updateReplyFormMode(mode, post) {
  var h3 = document.querySelector('#replySection h3');
  var btn = document.querySelector('#replyForm button[type="submit"]');
  var cancelBtn = document.getElementById('editCancelBtn');
  // Title field for thread editing
  var titleWrap = document.getElementById('editTitleWrap');
  var titleInput = document.getElementById('editTitleInput');

  if (mode === 'edit') {
    if (h3) h3.textContent = '编辑回复';
    if (btn) btn.innerHTML = '<i class="fa fa-save mr-1"></i>保存修改';
    if (!cancelBtn) {
      var a = document.createElement('button');
      a.type = 'button';
      a.id = 'editCancelBtn';
      a.className = 'bg-gray-100 text-black px-5 py-2 rounded-lg text-sm font-medium hover:bg-gray-200 transition-colors';
      a.textContent = '取消编辑';
      a.onclick = cancelEdit;
      if (btn && btn.parentNode) btn.parentNode.insertBefore(a, btn.nextSibling);
    }
    // Show title field for first post
    if (post && post.is_first === 1) {
      if (!titleWrap) {
        var div = document.createElement('div');
        div.id = 'editTitleWrap';
        div.className = 'mb-5';
        div.innerHTML = '<label class="block text-sm font-medium mb-2">标题</label><input type="text" id="editTitleInput" class="w-full border border-gray-200 rounded-lg px-4 py-3 text-sm outline-none focus:border-black transition-colors" value="' + escapeHtml(post.thread_title || '') + '">';
        var form = document.getElementById('replyForm');
        if (form) form.insertBefore(div, form.firstChild);
      }
    }
  } else {
    if (h3) h3.textContent = '回复帖子';
    if (btn) btn.innerHTML = '<i class="fa fa-reply mr-1"></i>发表回复';
    if (cancelBtn) cancelBtn.remove();
    if (titleWrap) titleWrap.remove();
  }
}

function cancelEdit() {
  var el = document.getElementById('replyContent');
  if (el && _editMode) {
    el.value = _editMode.original_content || '';
  }
  _editMode = null;
  updateReplyFormMode('reply', null);
}

// Override submitReply to handle edit mode
var _origSubmitReply = typeof submitReply === 'function' ? submitReply : null;

// Replace submitReply with version that handles edit mode
window.submitReply = function(e, threadId) {
  if (!_editMode) {
    // Normal reply - use original logic
    if (_origSubmitReply) { _origSubmitReply(e, threadId); }
    return;
  }
  e.preventDefault();
  var el = document.getElementById('replyContent');
  if (!el) return;
  var content = el.value.trim();
  if (!content) { showToast('内容不能为空'); return; }

  var btn = e.target.querySelector('button[type="submit"]');
  var origText = btn.innerHTML;
  btn.disabled = true;
  btn.innerHTML = '<i class="fa fa-spinner fa-spin mr-1"></i>保存中...';

  if (_editMode.is_first === 1) {
    // Edit thread (first post) - needs title
    var titleEl = document.getElementById('editTitleInput');
    var title = titleEl ? titleEl.value.trim() : '';
    if (!title) { showToast('标题不能为空'); btn.disabled = false; btn.innerHTML = origText; return; }
    fetch('/api/thread/' + _editMode.thread_id + '/edit', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'same-origin',
      body: JSON.stringify({ title: title, content: content }),
    })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) { showToast('修改成功'); setTimeout(function() { location.reload(); }, 600); }
      else { btn.disabled = false; btn.innerHTML = origText; showToast(res.error || '修改失败'); }
    })
    .catch(function() { btn.disabled = false; btn.innerHTML = origText; showToast('网络错误'); });
  } else {
    // Edit reply post
    fetch('/api/post/' + _editMode.post_id + '/edit', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      credentials: 'same-origin',
      body: JSON.stringify({ content: content }),
    })
    .then(function(r) { return r.json(); })
    .then(function(res) {
      if (res.ok) { showToast('修改成功'); setTimeout(function() { location.reload(); }, 600); }
      else { btn.disabled = false; btn.innerHTML = origText; showToast(res.error || '修改失败'); }
    })
    .catch(function() { btn.disabled = false; btn.innerHTML = origText; showToast('网络错误'); });
  }
};

// =====================================================================
// Report submission (frontend)
// =====================================================================
function submitReport(targetType, targetId) {
  var reasons = ['垃圾广告', '违规内容', '人身攻击', '虚假信息', '其他'];
  var reasonOptions = reasons.map(function(r, i) {
    return '<option value="' + r + '">' + r + '</option>';
  }).join('');
  var html = '<div class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center" id="reportDialog">' +
    '<div class="bg-white rounded-xl p-6 w-96 shadow-xl">' +
    '<h3 class="font-semibold text-lg mb-4">举报</h3>' +
    '<form onsubmit="doReport(event,\'' + targetType + '\',' + targetId + ')">' +
    '<div class="mb-3"><label class="text-sm text-gray-600">举报原因</label>' +
    '<select id="reportReason" class="w-full border rounded-lg px-3 py-2 text-sm mt-1">' + reasonOptions + '</select></div>' +
    '<div class="mb-4"><label class="text-sm text-gray-600">详细描述</label>' +
    '<textarea id="reportDesc" rows="3" class="w-full border rounded-lg px-3 py-2 text-sm mt-1" placeholder="可选..."></textarea></div>' +
    '<div class="flex gap-2">' +
    '<button type="submit" class="bg-red-500 text-white px-4 py-2 rounded-lg text-sm hover:bg-red-600">提交举报</button>' +
    '<button type="button" onclick="document.getElementById(\'reportDialog\').remove()" class="bg-gray-100 px-4 py-2 rounded-lg text-sm">取消</button>' +
    '</div></form></div></div>';
  document.body.insertAdjacentHTML('beforeend', html);
}

function doReport(e, targetType, targetId) {
  e.preventDefault();
  var reason = document.getElementById('reportReason').value;
  var desc = document.getElementById('reportDesc').value;
  fetch('/api/report', {
    method: 'POST',
    headers: { 'Content-Type': 'application/x-www-form-urlencoded' },
    credentials: 'same-origin',
    body: 'target_type=' + encodeURIComponent(targetType) + '&target_id=' + targetId + '&reason=' + encodeURIComponent(reason) + '&description=' + encodeURIComponent(desc)
  }).then(function(r) { return r.json(); })
  .then(function(res) {
    var dlg = document.getElementById('reportDialog');
    if (dlg) dlg.remove();
    showToast(res.ok ? '举报已提交' : (res.msg || '举报失败'));
  }).catch(function() { showToast('网络错误'); });
}
