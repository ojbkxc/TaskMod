/* ===== Global State ===== */
    let currentTab = 'dashboard';
    let chatWs = null;
    let mirrorWs = null;
    let mirrorDecoder = null;
    let logAutoRefreshInterval = null;
    let currentProviderId = '';
    let currentModel = '';
    let isChatStreaming = false;
    let ttsReplyEnabled = false; // 朗读AI回复开关
    const _loaded = {}; // 懒加载标记，避免重复请求

    /* ===== Tab Switching ===== */
    function showTab(name) {
        document.querySelectorAll('.side-tab').forEach((t, i) => {
            const tabs = ['dashboard','chat','mirror','library','tasks','scripts','tts','config','logs'];
            t.classList.toggle('side-tab-active', tabs[i] === name);
        });
        ['dashboard','chat','mirror','library','tasks','scripts','tts','config','logs'].forEach(id => {
            const el = document.getElementById('tab-' + id);
            if (el) el.style.display = id === name ? 'flex' : 'none';
        });
        currentTab = name;
        // 首次切换时加载数据，后续不重复请求
        if (name === 'dashboard') refreshStatus();
        if (name === 'tasks' && !_loaded.tasks) { _loaded.tasks = true; loadTasks(); }
        if (name === 'scripts' && !_loaded.scripts) { _loaded.scripts = true; loadScripts(); }
        if (name === 'tts' && !_loaded.tts) { _loaded.tts = true; loadTtsEngines(); loadTtsSettings(); }
        if (name === 'config' && !_loaded.config) { _loaded.config = true; loadEmailConfig(); loadMqttConfig(); loadVoiceSettings(); }
        if (name === 'logs') loadLogs();
        if (name === 'chat' && !_loaded.chat) { _loaded.chat = true; loadProviders(); }
        if (name === 'library' && !_loaded.library) {
            _loaded.library = true;
            loadMemories(); loadPresets(); loadSkills(); loadSavedItems();
            loadScenarios(); loadProjects(); loadMcpServers(); loadScreenshots(); loadPromptSettings();
        }
    }

    /* ===== Library Sub-tab Switching ===== */
    function showLibraryTab(name) {
        document.querySelectorAll('.lib-subtab').forEach(el => el.style.display = 'none');
        document.querySelectorAll('[id^="libtab-"]').forEach(el => el.classList.remove('ds-sub-tab-active'));
        document.getElementById(name).style.display = 'block';
        document.getElementById('libtab-' + name).classList.add('ds-sub-tab-active');
        // 子tab也做懒加载
        const key = 'lib_' + name;
        if (!_loaded[key]) {
            _loaded[key] = true;
            if (name === 'lib-memories') loadMemories();
            else if (name === 'lib-presets') loadPresets();
            else if (name === 'lib-skills') loadSkills();
            else if (name === 'lib-saved') loadSavedItems();
            else if (name === 'lib-scenarios') loadScenarios();
            else if (name === 'lib-projects') loadProjects();
            else if (name === 'lib-mcp') loadMcpServers();
            else if (name === 'lib-screenshots') loadScreenshots();
            else if (name === 'lib-prompt-ctrl') loadPromptSettings();
        }
    }

    /* ===== Theme Toggle ===== */
    function toggleTheme() {
        const root = document.documentElement;
        const isDark = root.classList.toggle('dark');
        root.style.colorScheme = isDark ? 'dark' : 'light';
    }

    /* ===== API Helpers ===== */
    async function apiGet(url) {
        try {
            const res = await fetch(url);
            const json = await res.json();
            json.ok = json.success;
            return json;
        } catch (e) {
            showToast('请求失败: ' + e.message, 'error');
            return { ok: false, success: false, message: e.message };
        }
    }

    async function apiPost(url, body) {
        try {
            const res = await fetch(url, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body)
            });
            const json = await res.json();
            json.ok = json.success;
            return json;
        } catch (e) {
            showToast('请求失败: ' + e.message, 'error');
            return { ok: false, success: false, message: e.message };
        }
    }

    async function apiPut(url, body) {
        try {
            const res = await fetch(url, {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(body)
            });
            const json = await res.json();
            json.ok = json.success;
            return json;
        } catch (e) {
            showToast('请求失败: ' + e.message, 'error');
            return { ok: false, success: false, message: e.message };
        }
    }

    async function apiDelete(url) {
        try {
            const res = await fetch(url, { method: 'DELETE' });
            const json = await res.json();
            json.ok = json.success;
            return json;
        } catch (e) {
            showToast('请求失败: ' + e.message, 'error');
            return { ok: false, success: false, message: e.message };
        }
    }

    /* ===== Toast Notifications ===== */
    function showToast(message, type = 'info') {
        const container = document.getElementById('toast-container');
        const toast = document.createElement('div');
        toast.className = 'toast toast-' + type;
        const icon = type === 'error' ? 'fa-exclamation-circle' : type === 'success' ? 'fa-check-circle' : 'fa-info-circle';
        toast.innerHTML = '<i class="fas ' + icon + '"></i> ' + escapeHtml(message);
        container.appendChild(toast);
        setTimeout(() => { toast.style.opacity = '0'; setTimeout(() => toast.remove(), 300); }, 3000);
    }

    function escapeHtml(text) {
        const d = document.createElement('div');
        d.textContent = text;
        return d.innerHTML;
    }

    /* ===== Dashboard ===== */
    async function refreshStatus() {
        const res = await apiGet('/api/status');
        if (res.ok && res.data) {
            const d = res.data;
            document.getElementById('status-battery').textContent = (d.battery ?? '--') + '%';
            document.getElementById('status-cpu').textContent = (d.cpu ?? '--') + '%';
            document.getElementById('status-memory').textContent = d.memory ? (d.memory + ' MB') : '-- MB';
            document.getElementById('status-uptime').textContent = d.uptime || '--';
        }
    }

    async function screenshot() {
        const res = await apiPost('/api/command', { command: 'screencap -p /sdcard/screenshot.png' });
        if (res.ok) showToast('截屏完成', 'success');
        else showToast(res.message || '截屏失败', 'error');
    }

    async function rebootDevice() {
        if (!confirm('确定要重启设备吗？')) return;
        const res = await apiPost('/api/command', { command: 'reboot' });
        if (res.ok) showToast('重启命令已发送', 'success');
        else showToast(res.message || '重启失败', 'error');
    }

    async function unlockScreen() {
        // 先唤醒屏幕
        await apiPost('/api/command', { command: 'input keyevent KEYCODE_WAKEUP' });
        // 等待300ms
        await new Promise(r => setTimeout(r, 300));
        // 上滑解锁：从底部80%滑到顶部30%
        const res = await apiPost('/api/command', { command: 'input swipe 540 1800 540 600 300' });
        if (res.ok) showToast('上滑解锁已执行', 'success');
        else showToast(res.message || '解锁失败', 'error');
    }

    /* ===== ADB 命令 ===== */
    async function sendAdbCommand() {
        const input = document.getElementById('adb-command-input');
        const cmd = input.value.trim();
        if (!cmd) return showToast('请输入命令', 'error');

        const resultDiv = document.getElementById('adb-command-result');
        const pre = resultDiv.querySelector('pre');
        resultDiv.style.display = 'block';
        pre.textContent = '执行中...';

        try {
            const res = await apiPost('/api/command', { command: cmd });
            if (res.ok) {
                const output = res.data || res.message || '(无输出)';
                pre.textContent = '$ ' + cmd + '\n' + output;
            } else {
                pre.textContent = '$ ' + cmd + '\n错误: ' + (res.message || '执行失败');
            }
        } catch (e) {
            pre.textContent = '$ ' + cmd + '\n请求异常: ' + e.message;
        }
    }

    function quickAdb(cmd) {
        document.getElementById('adb-command-input').value = cmd;
        sendAdbCommand();
    }

    /* ===== 设备工具 (剪贴板/上传/设备信息) ===== */
    async function loadDeviceInfo() {
        const container = document.getElementById('device-info-container');
        const el = document.getElementById('device-info');
        container.style.display = 'block';
        el.innerHTML = '<p style="color:var(--ds-text-secondary);">加载中...</p>';
        try {
            const res = await apiGet('/api/device/info');
            if (!res.success) { el.innerHTML = '<p style="color:var(--ds-text-secondary);">获取失败</p>'; return; }
            const info = res.data;
            const icons = { model:'📱', android_version:'🤖', screen_size:'🖥️', battery:'🔋', ip:'🌐', storage:'💾' };
            const labels = { model:'设备型号', android_version:'Android版本', screen_size:'屏幕分辨率', battery:'电池', ip:'IP地址', storage:'存储空间' };
            el.innerHTML = Object.entries(info).map(([k,v]) =>
                '<div style="padding:12px;text-align:center;background:var(--ds-bg);border-radius:8px;border:1px solid var(--ds-border);">' +
                    '<div style="font-size:24px;">' + (icons[k]||'📊') + '</div>' +
                    '<div style="font-size:16px;font-weight:bold;margin:5px 0;">' + (v||'N/A') + '</div>' +
                    '<div style="color:var(--ds-text-secondary);font-size:12px;">' + (labels[k]||k) + '</div>' +
                '</div>'
            ).join('');
        } catch(e) {
            el.innerHTML = '<p style="color:var(--ds-text-secondary);">请求失败: ' + e.message + '</p>';
        }
    }

    async function getDeviceClipboard() {
        const container = document.getElementById('clipboard-container');
        container.style.display = 'block';
        try {
            const res = await apiGet('/api/device/clipboard');
            if (res.success) {
                document.getElementById('clipboard-text').value = res.data || '';
                showToast('已获取设备剪贴板');
            } else {
                showToast(res.message || '获取失败', 'error');
            }
        } catch(e) {
            showToast('获取剪贴板失败: ' + e.message, 'error');
        }
    }

    async function setDeviceClipboard() {
        const container = document.getElementById('clipboard-container');
        container.style.display = 'block';
        const text = document.getElementById('clipboard-text').value;
        if (!text) { showToast('请输入文本', 'error'); return; }
        try {
            const res = await fetch('/api/device/clipboard', {
                method: 'PUT',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify({ text })
            });
            const data = await res.json();
            showToast(data.success ? '已设置设备剪贴板' : (data.message || '设置失败'), data.success ? 'success' : 'error');
        } catch(e) {
            showToast('设置剪贴板失败: ' + e.message, 'error');
        }
    }

    function handleFileSelect(files) {
        if (files.length > 0) uploadFilesToDevice(files);
    }

    async function uploadFilesToDevice(files) {
        const progress = document.getElementById('file-upload-progress');
        for (const file of files) {
            progress.innerHTML = '<p style="color:var(--ds-text-secondary);">正在上传: ' + file.name + ' (' + (file.size/1024).toFixed(1) + 'KB)...</p>';
            const reader = new FileReader();
            reader.onload = async (e) => {
                const base64 = e.target.result.split(',')[1];
                try {
                    const res = await apiPost('/api/device/upload-file', { file_base64: base64, filename: file.name });
                    if (res.success) {
                        progress.innerHTML += '<p style="color:#28a745;">✓ ' + file.name + ' → ' + res.data + '</p>';
                    } else {
                        progress.innerHTML += '<p style="color:#dc3545;">✗ ' + file.name + ': ' + (res.message || '上传失败') + '</p>';
                    }
                } catch(err) {
                    progress.innerHTML += '<p style="color:#dc3545;">✗ ' + file.name + ': ' + err.message + '</p>';
                }
            };
            reader.readAsDataURL(file);
            await new Promise(r => setTimeout(r, 500));
        }
    }

    /* ===== AI Chat ===== */
    async function loadProviders() {
        const res = await apiGet('/api/ai/providers');
        if (res.ok && res.data) {
            updateProviderSelector(res.data);
        }
    }

    function updateProviderSelector(providers) {
        if (!providers || providers.length === 0) {
            document.getElementById('current-provider-name').textContent = '未配置';
            document.getElementById('current-model-name').textContent = '--';
            return;
        }
        if (!currentProviderId && providers.length > 0) {
            currentProviderId = providers[0].id;
            currentModel = providers[0].model || '';
        }
        const p = providers.find(x => x.id === currentProviderId) || providers[0];
        currentProviderId = p.id;
        currentModel = p.model || '';
        document.getElementById('current-provider-name').textContent = p.name || '未知';
        document.getElementById('current-model-name').textContent = currentModel || '--';
    }

    function clearChat() {
        const container = document.getElementById('chat-messages');
        container.innerHTML = '<div class="ds-chat-empty"><div class="ds-empty-state-icon"><svg class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="1.8"><path stroke-linecap="round" stroke-linejoin="round" d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" /></svg></div><div class="ds-empty-state-title">开始与 AI 对话</div><div class="ds-empty-state-description">选择一个AI提供商，然后输入消息控制设备</div></div>';
        currentAssistantEl = null;
        isChatStreaming = false;
        if (chatWs && chatWs.readyState === WebSocket.OPEN) {
            chatWs.close();
            chatWs = null;
        }
        showToast('已开始新对话', 'info');
    }

    function openProviderModal() {
        document.getElementById('provider-modal').style.display = 'flex';
        loadProviderList();
    }

    async function loadProviderList() {
        const res = await apiGet('/api/ai/providers');
        const container = document.getElementById('provider-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            container.innerHTML = '<div class="ds-empty-state"><p>暂无提供商，添加一个吧</p></div>';
            return;
        }
        container.innerHTML = res.data.map(p => {
            const isActive = p.id === currentProviderId;
            return '<div class="provider-option' + (isActive ? ' provider-option-active' : '') + '" style="display:flex;justify-content:space-between;align-items:center;padding:10px;border:1px solid var(--ds-border);border-radius:8px;cursor:pointer;" onclick="selectProvider(\'' + p.id + '\',\'' + escapeHtml(p.model || '') + '\')">' +
                '<div><strong>' + escapeHtml(p.name) + '</strong> <span class="ds-badge-success">' + escapeHtml(p.model || '--') + '</span></div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:0.7rem;padding:2px 8px;" onclick="event.stopPropagation();editProvider(\'' + p.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:0.7rem;padding:2px 8px;" onclick="event.stopPropagation();deleteProvider(\'' + p.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div>';
        }).join('');
    }

    function selectProvider(id, model) {
        currentProviderId = id;
        currentModel = model;
        document.getElementById('current-provider-name').textContent = '已选择';
        document.getElementById('current-model-name').textContent = model || '--';
        loadProviders();
        showToast('已切换提供商', 'success');
        document.getElementById('provider-modal').style.display = 'none';
    }

    async function editProvider(id) {
        const res = await apiGet('/api/ai/providers');
        if (!res.ok) return;
        const p = (res.data || []).find(x => x.id === id);
        if (!p) return;
        document.getElementById('prov-name').value = p.name || '';
        document.getElementById('prov-url').value = p.base_url || p.api_url || '';
        document.getElementById('prov-key').value = p.api_key || '';
        document.getElementById('prov-model').value = p.model || '';
        document.getElementById('prov-edit-id').value = id;
    }

    async function saveProvider() {
        const body = {
            name: document.getElementById('prov-name').value.trim(),
            base_url: document.getElementById('prov-url').value.trim(),
            api_key: document.getElementById('prov-key').value.trim(),
            model: document.getElementById('prov-model').value.trim()
        };
        if (!body.name || !body.base_url) return showToast('名称和 API 地址为必填', 'error');
        const editId = document.getElementById('prov-edit-id').value;
        let res;
        if (editId) {
            res = await apiPut('/api/ai/providers/' + editId, body);
        } else {
            res = await apiPost('/api/ai/providers', body);
        }
        if (res.ok) {
            showToast('提供商已保存', 'success');
            document.getElementById('prov-edit-id').value = '';
            ['prov-name','prov-url','prov-key','prov-model'].forEach(id => document.getElementById(id).value = '');
            loadProviderList();
            loadProviders();
        } else {
            showToast(res.message || '保存失败', 'error');
        }
    }

    async function deleteProvider(id) {
        if (!confirm('确定删除此提供商？')) return;
        const res = await apiDelete('/api/ai/providers/' + id);
        if (res.ok) {
            showToast('已删除', 'success');
            if (currentProviderId === id) { currentProviderId = ''; currentModel = ''; }
            loadProviderList();
            loadProviders();
        } else {
            showToast(res.message || '删除失败', 'error');
        }
    }

    /* ===== Library: Memories ===== */
    async function loadMemories() {
        const q = document.getElementById('lib-memory-search')?.value || '';
        const res = await apiGet('/api/ai/memories' + (q ? '?q=' + encodeURIComponent(q) : ''));
        const list = document.getElementById('lib-memories-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无记忆</p></div>';
            return;
        }
        list.innerHTML = res.data.map(m => {
            const tags = (m.tags || []).map(t => '<span class="ds-badge">' + escapeHtml(t) + '</span>').join('');
            return '<div class="ds-card" style="padding:12px;margin-bottom:8px;">' +
                '<div style="display:flex;justify-content:space-between;align-items:flex-start;">' +
                '<div style="flex:1;"><strong>' + escapeHtml(m.name || '未命名') + '</strong>' +
                '<p style="font-size:12px;color:var(--ds-text-secondary);margin-top:4px;">' + escapeHtml(m.content.substring(0, 100)) + '</p>' +
                '<div style="margin-top:6px;">' + tags + '</div></div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="editMemory(\'' + m.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:11px;padding:4px 8px;" onclick="deleteMemory(\'' + m.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div></div>';
        }).join('');
    }

    async function editMemory(id) {
        const res = await apiGet('/api/ai/memories');
        if (!res.ok) return;
        const m = (res.data || []).find(x => x.id === id);
        if (m) showMemoryModal(m);
    }

    async function deleteMemory(id) {
        if (!confirm('确定删除此记忆？')) return;
        const res = await apiDelete('/api/ai/memories/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadMemories(); }
        else showToast(res.message || '删除失败', 'error');
    }

    function showMemoryModal(m) {
        document.getElementById('memory-modal-title').textContent = m ? '编辑记忆' : '新建记忆';
        document.getElementById('memory-name').value = m?.name || '';
        document.getElementById('memory-content').value = m?.content || '';
        document.getElementById('memory-tags').value = (m?.tags || []).join(', ');
        document.getElementById('memory-edit-id').value = m?.id || '';
        document.getElementById('memory-modal').style.display = 'flex';
    }

    async function saveMemory() {
        const body = {
            name: document.getElementById('memory-name').value.trim(),
            content: document.getElementById('memory-content').value.trim(),
            tags: document.getElementById('memory-tags').value.split(',').map(t => t.trim()).filter(Boolean)
        };
        if (!body.content) return showToast('内容为必填', 'error');
        const editId = document.getElementById('memory-edit-id').value;
        const res = editId ? await apiPut('/api/ai/memories/' + editId, body) : await apiPost('/api/ai/memories', body);
        if (res.ok) {
            showToast('记忆已保存', 'success');
            document.getElementById('memory-modal').style.display = 'none';
            loadMemories();
        } else {
            showToast(res.message || '保存失败', 'error');
        }
    }

    /* ===== Library: Presets ===== */
    async function loadPresets() {
        const res = await apiGet('/api/ai/presets');
        const list = document.getElementById('lib-presets-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无预设</p></div>';
            return;
        }
        list.innerHTML = res.data.map(p => {
            return '<div class="ds-card" style="padding:12px;margin-bottom:8px;">' +
                '<div style="display:flex;justify-content:space-between;align-items:flex-start;">' +
                '<div style="flex:1;"><strong>' + escapeHtml(p.name) + '</strong>' +
                '<p style="font-size:12px;color:var(--ds-text-secondary);margin-top:4px;">' + escapeHtml(p.content.substring(0, 100)) + '</p></div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="editPreset(\'' + p.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:11px;padding:4px 8px;" onclick="deletePreset(\'' + p.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div></div>';
        }).join('');
    }

    async function editPreset(id) {
        const res = await apiGet('/api/ai/presets');
        if (!res.ok) return;
        const p = (res.data || []).find(x => x.id === id);
        if (p) showPresetModal(p);
    }

    async function deletePreset(id) {
        if (!confirm('确定删除此预设？')) return;
        const res = await apiDelete('/api/ai/presets/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadPresets(); }
        else showToast(res.message || '删除失败', 'error');
    }

    function showPresetModal(p) {
        document.getElementById('preset-modal-title').textContent = p ? '编辑预设' : '新建预设';
        document.getElementById('preset-name').value = p?.name || '';
        document.getElementById('preset-content').value = p?.content || '';
        document.getElementById('preset-edit-id').value = p?.id || '';
        document.getElementById('preset-modal').style.display = 'flex';
    }

    async function savePreset() {
        const body = {
            name: document.getElementById('preset-name').value.trim(),
            content: document.getElementById('preset-content').value.trim()
        };
        if (!body.name || !body.content) return showToast('名称和内容为必填', 'error');
        const editId = document.getElementById('preset-edit-id').value;
        const res = editId ? await apiPut('/api/ai/presets/' + editId, body) : await apiPost('/api/ai/presets', body);
        if (res.ok) {
            showToast('预设已保存', 'success');
            document.getElementById('preset-modal').style.display = 'none';
            loadPresets();
        } else {
            showToast(res.message || '保存失败', 'error');
        }
    }

    /* ===== Library: Skills ===== */
    async function loadSkills() {
        const res = await apiGet('/api/ai/skills');
        const list = document.getElementById('lib-skills-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无技能</p></div>';
            return;
        }
        list.innerHTML = res.data.map(s => {
            return '<div class="ds-card" style="padding:12px;margin-bottom:8px;">' +
                '<div style="display:flex;justify-content:space-between;align-items:flex-start;">' +
                '<div style="flex:1;"><strong>' + escapeHtml(s.name) + '</strong>' +
                '<p style="font-size:12px;color:var(--ds-text-secondary);margin-top:4px;">' + escapeHtml(s.description || '') + '</p></div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="editSkill(\'' + s.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:11px;padding:4px 8px;" onclick="deleteSkill(\'' + s.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div></div>';
        }).join('');
    }

    async function editSkill(id) {
        const res = await apiGet('/api/ai/skills');
        if (!res.ok) return;
        const s = (res.data || []).find(x => x.id === id);
        if (s) showSkillModal(s);
    }

    async function deleteSkill(id) {
        if (!confirm('确定删除此技能？')) return;
        const res = await apiDelete('/api/ai/skills/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadSkills(); }
        else showToast(res.message || '删除失败', 'error');
    }

    function showSkillModal(s) {
        document.getElementById('skill-modal-title').textContent = s ? '编辑技能' : '新建技能';
        document.getElementById('skill-name').value = s?.name || '';
        document.getElementById('skill-desc').value = s?.description || '';
        document.getElementById('skill-prompt').value = s?.prompt_template || '';
        document.getElementById('skill-edit-id').value = s?.id || '';
        document.getElementById('skill-modal').style.display = 'flex';
    }

    async function saveSkill() {
        const body = {
            name: document.getElementById('skill-name').value.trim(),
            description: document.getElementById('skill-desc').value.trim(),
            prompt_template: document.getElementById('skill-prompt').value.trim()
        };
        if (!body.name || !body.prompt_template) return showToast('名称和提示词为必填', 'error');
        const editId = document.getElementById('skill-edit-id').value;
        const res = editId ? await apiPut('/api/ai/skills/' + editId, body) : await apiPost('/api/ai/skills', body);
        if (res.ok) {
            showToast('技能已保存', 'success');
            document.getElementById('skill-modal').style.display = 'none';
            loadSkills();
        } else {
            showToast(res.message || '保存失败', 'error');
        }
    }

    /* ===== Library: Saved Items ===== */
    async function loadSavedItems() {
        const res = await apiGet('/api/ai/saved');
        const list = document.getElementById('lib-saved-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无保存项</p></div>';
            return;
        }
        list.innerHTML = res.data.map(s => {
            return '<div class="ds-card" style="padding:12px;margin-bottom:8px;">' +
                '<div style="display:flex;justify-content:space-between;align-items:flex-start;">' +
                '<div style="flex:1;"><strong>' + escapeHtml(s.title) + '</strong>' +
                '<p style="font-size:12px;color:var(--ds-text-secondary);margin-top:4px;">' + escapeHtml(s.content.substring(0, 100)) + '</p></div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="editSavedItem(\'' + s.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:11px;padding:4px 8px;" onclick="deleteSavedItem(\'' + s.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div></div>';
        }).join('');
    }

    async function editSavedItem(id) {
        const res = await apiGet('/api/ai/saved');
        if (!res.ok) return;
        const s = (res.data || []).find(x => x.id === id);
        if (s) showSavedItemModal(s);
    }

    async function deleteSavedItem(id) {
        if (!confirm('确定删除此保存项？')) return;
        const res = await apiDelete('/api/ai/saved/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadSavedItems(); }
        else showToast(res.message || '删除失败', 'error');
    }

    function showSavedItemModal(s) {
        document.getElementById('saved-modal-title').textContent = s ? '编辑保存项' : '新建保存项';
        document.getElementById('saved-title').value = s?.title || '';
        document.getElementById('saved-content').value = s?.content || '';
        document.getElementById('saved-edit-id').value = s?.id || '';
        document.getElementById('saved-modal').style.display = 'flex';
    }

    async function saveSavedItem() {
        const body = {
            title: document.getElementById('saved-title').value.trim(),
            content: document.getElementById('saved-content').value.trim()
        };
        if (!body.title || !body.content) return showToast('标题和内容为必填', 'error');
        const editId = document.getElementById('saved-edit-id').value;
        const res = editId ? await apiPut('/api/ai/saved/' + editId, body) : await apiPost('/api/ai/saved', body);
        if (res.ok) {
            showToast('保存项已保存', 'success');
            document.getElementById('saved-modal').style.display = 'none';
            loadSavedItems();
        } else {
            showToast(res.message || '保存失败', 'error');
        }
    }

    /* ===== Scenarios ===== */
    async function loadScenarios() {
        const res = await apiGet('/api/ai/scenarios');
        const list = document.getElementById('lib-scenarios-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无场景模板</p></div>';
            return;
        }
        list.innerHTML = res.data.map(s => {
            return '<div class="ds-card" style="padding:12px;margin-bottom:8px;">' +
                '<div style="display:flex;justify-content:space-between;align-items:flex-start;">' +
                '<div style="flex:1;"><strong>' + escapeHtml(s.name) + '</strong>' +
                (s.description ? '<p style="font-size:12px;color:var(--ds-text-secondary);margin-top:4px;">' + escapeHtml(s.description) + '</p>' : '') +
                '<p style="font-size:11px;color:var(--ds-text-muted);margin-top:4px;word-break:break-all;">' + escapeHtml(s.prompt_template.substring(0, 120)) + '</p></div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="useScenario(\'' + s.id + '\')"><i class="fas fa-play"></i></button>' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="editScenario(\'' + s.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:11px;padding:4px 8px;" onclick="deleteScenario(\'' + s.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div></div>';
        }).join('');
    }

    async function useScenario(id) {
        const res = await apiGet('/api/ai/scenarios');
        if (!res.ok) return;
        const s = (res.data || []).find(x => x.id === id);
        if (s) {
            const text = prompt('请输入要处理的文本:');
            if (text) {
                showTab('chat');
                document.getElementById('chat-input').value = s.prompt_template.replace(/\{text\}/g, text);
                sendChatMessage();
            }
        }
    }

    async function editScenario(id) {
        const res = await apiGet('/api/ai/scenarios');
        if (!res.ok) return;
        const s = (res.data || []).find(x => x.id === id);
        if (s) showScenarioModal(s);
    }

    async function deleteScenario(id) {
        if (!confirm('确定删除此场景？')) return;
        const res = await apiDelete('/api/ai/scenarios/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadScenarios(); }
        else showToast(res.message || '删除失败', 'error');
    }

    function showScenarioModal(s) {
        document.getElementById('scenario-modal-title').textContent = s ? '编辑场景' : '新建场景';
        document.getElementById('scenario-name').value = s?.name || '';
        document.getElementById('scenario-desc').value = s?.description || '';
        document.getElementById('scenario-prompt').value = s?.prompt_template || '';
        document.getElementById('scenario-edit-id').value = s?.id || '';
        document.getElementById('scenario-modal').style.display = 'flex';
    }

    async function saveScenario() {
        const body = {
            name: document.getElementById('scenario-name').value.trim(),
            description: document.getElementById('scenario-desc').value.trim(),
            prompt_template: document.getElementById('scenario-prompt').value.trim()
        };
        if (!body.name || !body.prompt_template) return showToast('名称和模板为必填', 'error');
        const editId = document.getElementById('scenario-edit-id').value;
        const res = editId ? await apiPut('/api/ai/scenarios/' + editId, body) : await apiPost('/api/ai/scenarios', body);
        if (res.ok) { showToast('场景已保存', 'success'); document.getElementById('scenario-modal').style.display = 'none'; loadScenarios(); }
        else showToast(res.message || '保存失败', 'error');
    }

    /* ===== Projects ===== */
    async function loadProjects() {
        const res = await apiGet('/api/ai/projects');
        const list = document.getElementById('lib-projects-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无项目</p></div>';
            return;
        }
        list.innerHTML = res.data.map(p => {
            return '<div class="ds-card" style="padding:12px;margin-bottom:8px;">' +
                '<div style="display:flex;justify-content:space-between;align-items:flex-start;">' +
                '<div style="flex:1;"><strong>' + escapeHtml(p.name) + '</strong>' +
                (p.description ? '<p style="font-size:12px;color:var(--ds-text-secondary);margin-top:4px;">' + escapeHtml(p.description) + '</p>' : '') +
                '</div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="editProject(\'' + p.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:11px;padding:4px 8px;" onclick="deleteProject(\'' + p.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div></div>';
        }).join('');
    }

    async function editProject(id) {
        const res = await apiGet('/api/ai/projects');
        if (!res.ok) return;
        const p = (res.data || []).find(x => x.id === id);
        if (p) showProjectModal(p);
    }

    async function deleteProject(id) {
        if (!confirm('确定删除此项目？')) return;
        const res = await apiDelete('/api/ai/projects/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadProjects(); }
        else showToast(res.message || '删除失败', 'error');
    }

    function showProjectModal(p) {
        document.getElementById('project-modal-title').textContent = p ? '编辑项目' : '新建项目';
        document.getElementById('project-name').value = p?.name || '';
        document.getElementById('project-desc').value = p?.description || '';
        document.getElementById('project-instructions').value = p?.instructions || '';
        document.getElementById('project-edit-id').value = p?.id || '';
        document.getElementById('project-modal').style.display = 'flex';
    }

    async function saveProject() {
        const body = {
            name: document.getElementById('project-name').value.trim(),
            description: document.getElementById('project-desc').value.trim(),
            instructions: document.getElementById('project-instructions').value.trim()
        };
        if (!body.name) return showToast('名称为必填', 'error');
        const editId = document.getElementById('project-edit-id').value;
        const res = editId ? await apiPut('/api/ai/projects/' + editId, body) : await apiPost('/api/ai/projects', body);
        if (res.ok) { showToast('项目已保存', 'success'); document.getElementById('project-modal').style.display = 'none'; loadProjects(); }
        else showToast(res.message || '保存失败', 'error');
    }

    /* ===== MCP Servers ===== */
    async function loadMcpServers() {
        const res = await apiGet('/api/ai/mcp');
        const list = document.getElementById('lib-mcp-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无MCP服务器</p></div>';
            return;
        }
        list.innerHTML = res.data.map(m => {
            return '<div class="ds-card" style="padding:12px;margin-bottom:8px;">' +
                '<div style="display:flex;justify-content:space-between;align-items:flex-start;">' +
                '<div style="flex:1;"><strong>' + escapeHtml(m.name) + '</strong>' +
                '<span class="ds-badge" style="margin-left:8px;">' + escapeHtml(m.transport) + '</span>' +
                '<p style="font-size:12px;color:var(--ds-text-secondary);margin-top:4px;">' + escapeHtml(m.command || m.url || '') + '</p></div>' +
                '<div style="display:flex;gap:6px;">' +
                '<button class="ds-btn-secondary" style="font-size:11px;padding:4px 8px;" onclick="editMcpServer(\'' + m.id + '\')"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:11px;padding:4px 8px;" onclick="deleteMcpServer(\'' + m.id + '\')"><i class="fas fa-trash"></i></button>' +
                '</div></div></div>';
        }).join('');
    }

    async function editMcpServer(id) {
        const res = await apiGet('/api/ai/mcp');
        if (!res.ok) return;
        const m = (res.data || []).find(x => x.id === id);
        if (m) showMcpModal(m);
    }

    async function deleteMcpServer(id) {
        if (!confirm('确定删除此MCP服务器？')) return;
        const res = await apiDelete('/api/ai/mcp/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadMcpServers(); }
        else showToast(res.message || '删除失败', 'error');
    }

    function showMcpModal(m) {
        document.getElementById('mcp-modal-title').textContent = m ? '编辑 MCP 服务器' : '添加 MCP 服务器';
        document.getElementById('mcp-name').value = m?.name || '';
        document.getElementById('mcp-transport').value = m?.transport || 'stdio';
        document.getElementById('mcp-command').value = m?.command || '';
        document.getElementById('mcp-url').value = m?.url || '';
        document.getElementById('mcp-edit-id').value = m?.id || '';
        toggleMcpFields();
        document.getElementById('mcp-modal').style.display = 'flex';
    }

    function toggleMcpFields() {
        const t = document.getElementById('mcp-transport').value;
        document.getElementById('mcp-command-group').style.display = t === 'stdio' ? 'block' : 'none';
        document.getElementById('mcp-url-group').style.display = t !== 'stdio' ? 'block' : 'none';
    }

    async function saveMcpServer() {
        const transport = document.getElementById('mcp-transport').value;
        const body = {
            name: document.getElementById('mcp-name').value.trim(),
            transport,
            command: transport === 'stdio' ? document.getElementById('mcp-command').value.trim() : '',
            url: transport !== 'stdio' ? document.getElementById('mcp-url').value.trim() : ''
        };
        if (!body.name) return showToast('名称为必填', 'error');
        const editId = document.getElementById('mcp-edit-id').value;
        const res = editId ? await apiPut('/api/ai/mcp/' + editId, body) : await apiPost('/api/ai/mcp', body);
        if (res.ok) { showToast('MCP服务器已保存', 'success'); document.getElementById('mcp-modal').style.display = 'none'; loadMcpServers(); }
        else showToast(res.message || '保存失败', 'error');
    }

    /* ===== Screenshots ===== */
    async function loadScreenshots() {
        const res = await apiGet('/api/screenshots');
        const list = document.getElementById('lib-screenshots-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><p>暂无截图</p></div>';
            return;
        }
        list.innerHTML = res.data.map(s => {
            return '<div class="ds-card" style="padding:8px;cursor:pointer;" onclick="window.open(\'/api/screenshots/' + s.filename + '\')">' +
                '<img src="/api/screenshots/' + s.filename + '" style="width:100%;height:120px;object-fit:cover;border-radius:6px;" loading="lazy">' +
                '<p style="font-size:11px;color:var(--ds-text-secondary);margin-top:6px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;">' + escapeHtml(s.filename) + '</p>' +
                '<div style="display:flex;gap:4px;margin-top:4px;">' +
                '<button class="ds-btn-secondary" style="font-size:10px;padding:2px 6px;flex:1;" onclick="event.stopPropagation();analyzeScreenshot(\'' + s.filename + '\')">AI分析</button>' +
                '<button class="ds-btn-danger" style="font-size:10px;padding:2px 6px;flex:1;" onclick="event.stopPropagation();deleteScreenshot(\'' + s.filename + '\')">删除</button>' +
                '</div></div>';
        }).join('');
    }

    async function takeScreenshot() {
        const res = await apiPost('/api/screenshots/take', {});
        if (res.ok) { showToast('截图已保存', 'success'); loadScreenshots(); }
        else showToast(res.message || '截图失败', 'error');
    }

    async function deleteScreenshot(filename) {
        if (!confirm('确定删除此截图？')) return;
        const res = await apiDelete('/api/screenshots/' + filename);
        if (res.ok) { showToast('已删除', 'success'); loadScreenshots(); }
        else showToast(res.message || '删除失败', 'error');
    }

    async function analyzeScreenshot(filename) {
        showTab('chat');
        addMessageToUI('user', '[截图分析] ' + filename);
        const res = await apiPost('/api/ai/screenshot', { filename });
        if (res.ok && res.data?.message) {
            addMessageToUI('assistant', res.data.message);
        } else {
            addMessageToUI('assistant', '截图分析失败: ' + (res.message || '未知错误'));
        }
    }

    /* ===== Prompt Settings ===== */
    async function loadPromptSettings() {
        const res = await apiGet('/api/ai/prompt-settings');
        if (!res.ok || !res.data) return;
        const d = res.data;
        document.getElementById('prompt-memory-enabled').checked = d.memory_enabled !== false;
        document.getElementById('prompt-system-enabled').checked = d.system_prompt_enabled !== false;
        document.getElementById('prompt-preset-frequency').value = d.preset_frequency || 'first';
        document.getElementById('prompt-force-language').value = d.force_language || 'auto';
    }

    async function savePromptSettings() {
        const body = {
            memory_enabled: document.getElementById('prompt-memory-enabled').checked,
            system_prompt_enabled: document.getElementById('prompt-system-enabled').checked,
            preset_frequency: document.getElementById('prompt-preset-frequency').value,
            force_language: document.getElementById('prompt-force-language').value
        };
        const res = await apiPut('/api/ai/prompt-settings', body);
        if (res.ok) showToast('设置已保存', 'success');
        else showToast(res.message || '保存失败', 'error');
    }

    /* ===== Chat: Screenshot Analyze ===== */
    async function screenshotAnalyze() {
        const res = await apiPost('/api/screenshots/take', {});
        if (!res.ok) return showToast('截图失败', 'error');
        const filename = res.data?.filename;
        if (!filename) return showToast('未获取到截图文件名', 'error');
        addMessageToUI('user', '[截图分析] 请分析当前屏幕');
        const analysis = await apiPost('/api/ai/screenshot', { filename });
        if (analysis.ok && analysis.data?.message) {
            addMessageToUI('assistant', analysis.data.message);
        } else {
            addMessageToUI('assistant', '截图分析失败: ' + (analysis.message || '未知错误'));
        }
    }

    /* ===== Chat: Export ===== */
    async function exportChat() {
        const format = prompt('导出格式 (md/json):', 'md');
        if (!format) return;
        const messages = [];
        document.querySelectorAll('#chat-messages .ds-chat-message').forEach(el => {
            const role = el.classList.contains('ds-chat-message-user') ? 'user' : 'assistant';
            const content = el.querySelector('.ds-chat-message-content')?.textContent || '';
            messages.push({ role, content });
        });
        if (messages.length === 0) return showToast('没有可导出的消息', 'error');
        const res = await apiPost('/api/ai/export', { messages, format });
        if (res.ok && res.data?.content) {
            const blob = new Blob([res.data.content], { type: 'text/plain' });
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'chat-export.' + format;
            a.click();
            showToast('导出成功', 'success');
        } else {
            showToast(res.message || '导出失败', 'error');
        }
    }

    function connectChatWs() {
        if (chatWs && chatWs.readyState <= 1) return;
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        chatWs = new WebSocket(proto + '//' + location.host + '/ws/ai-chat');
        chatWs.onopen = () => { /* ready */ };
        chatWs.onmessage = (evt) => {
            try {
                const msg = JSON.parse(evt.data);
                handleChatMessage(msg);
            } catch (e) { /* ignore non-JSON */ }
        };
        chatWs.onclose = () => { isChatStreaming = false; };
        chatWs.onerror = () => { showToast('AI 连接错误', 'error'); isChatStreaming = false; };
    }

    let currentAssistantEl = null;

    function handleChatMessage(msg) {
        const container = document.getElementById('chat-messages');
        const empty = container.querySelector('.ds-empty-state');
        if (empty) empty.remove();

        if (msg.type === 'chunk') {
            if (!currentAssistantEl) {
                currentAssistantEl = createMessageRow('assistant', '');
                container.appendChild(currentAssistantEl);
            }
            const body = currentAssistantEl.querySelector('.ds-chat-message-content') || currentAssistantEl.querySelector('.markdown-body');
            if (body) body.innerHTML += escapeHtml(msg.content);
            scrollChatToBottom();
        } else if (msg.type === 'thinking') {
            if (!currentAssistantEl) {
                currentAssistantEl = createMessageRow('assistant', '');
                container.appendChild(currentAssistantEl);
            }
            const body = currentAssistantEl.querySelector('.ds-chat-message-content') || currentAssistantEl.querySelector('.markdown-body');
            if (body) {
                let thinkingEl = body.querySelector('.ds-chat-thinking');
                if (!thinkingEl) {
                    thinkingEl = document.createElement('details');
                    thinkingEl.className = 'ds-chat-thinking';
                    thinkingEl.open = true;
                    thinkingEl.innerHTML = '<summary>思考中...</summary><div></div>';
                    body.prepend(thinkingEl);
                }
                const content = thinkingEl.querySelector('div');
                if (content) content.innerHTML += escapeHtml(msg.content);
            }
            scrollChatToBottom();
        } else if (msg.type === 'tool_call') {
            // 兼容旧格式：单个tool_call
            const row = createMessageRow('assistant', '');
            const body = row.querySelector('.markdown-body');
            if (body) {
                body.innerHTML = '<div class="chat-tool-call"><div class="tool-name"><i class="fas fa-cog fa-spin"></i> ' + escapeHtml(msg.name) + '</div><div class="tool-args">' + escapeHtml(typeof msg.arguments === 'string' ? msg.arguments : JSON.stringify(msg.arguments || {})) + '</div></div>';
            }
            container.appendChild(row);
            scrollChatToBottom();
        } else if (msg.type === 'tool_result') {
            // 新格式：批量results数组
            if (msg.results && Array.isArray(msg.results)) {
                msg.results.forEach(function(tr) {
                    const name = tr.name || '工具';
                    const result = tr.content || tr.result || '';
                    const row = createMessageRow('assistant', '');
                    const body = row.querySelector('.markdown-body');
                    if (body) {
                        body.innerHTML = '<div class="chat-tool-result"><div class="tool-name"><i class="fas fa-check-circle"></i> ' + escapeHtml(name) + '</div><div class="tool-output">' + escapeHtml(typeof result === 'string' ? result : JSON.stringify(result)) + '</div></div>';
                    }
                    container.appendChild(row);
                });
            } else {
                // 兼容旧格式：单个tool_result
                const row = createMessageRow('assistant', '');
                const body = row.querySelector('.markdown-body');
                if (body) {
                    body.innerHTML = '<div class="chat-tool-result"><div class="tool-name"><i class="fas fa-check-circle"></i> ' + escapeHtml(msg.name || '工具') + '</div><div class="tool-output">' + escapeHtml(typeof msg.result === 'string' ? msg.result : JSON.stringify(msg.result || '')) + '</div></div>';
                }
                container.appendChild(row);
            }
            scrollChatToBottom();
        } else if (msg.type === 'done' || msg.type === 'stream_end') {
            if (currentAssistantEl) {
                const body = currentAssistantEl.querySelector('.ds-chat-message-content') || currentAssistantEl.querySelector('.markdown-body');
                if (body && typeof marked !== 'undefined') {
                    try { body.innerHTML = marked.parse(body.textContent); } catch (e) { /* keep raw */ }
                }
                // 自动朗读AI回复
                if (ttsReplyEnabled && body) {
                    const text = body.textContent.trim();
                    if (text) ttsReadText(text);
                }
                // 添加朗读按钮到消息气泡
                if (body) {
                    const readBtn = document.createElement('button');
                    readBtn.className = 'ds-msg-read-btn';
                    readBtn.innerHTML = '<i class="fas fa-volume-up"></i>';
                    readBtn.title = '朗读此回复';
                    readBtn.style.cssText = 'position:absolute;top:4px;right:4px;background:rgba(0,0,0,0.05);border:none;border-radius:50%;width:24px;height:24px;cursor:pointer;font-size:11px;color:#888;display:flex;align-items:center;justify-content:center;';
                    readBtn.onclick = function() { ttsReadText(body.textContent.trim()); };
                    const bubble = body.closest('.ds-chat-message');
                    if (bubble) { bubble.style.position = 'relative'; bubble.appendChild(readBtn); }
                }
            }
            currentAssistantEl = null;
            isChatStreaming = false;
        } else if (msg.type === 'title') {
            // 对话标题自动生成
            if (msg.title) {
                console.log('对话标题:', msg.title);
            }
        } else if (msg.type === 'image') {
            // 图片消息
            if (msg.url) {
                const row = createMessageRow('assistant', '');
                const body = row.querySelector('.markdown-body');
                if (body) {
                    body.innerHTML = '<img src="' + escapeHtml(msg.url) + '" style="max-width:100%;border-radius:8px;">';
                }
                container.appendChild(row);
                scrollChatToBottom();
            }
        } else if (msg.type === 'error') {
            const row = document.createElement('div');
            row.className = 'ds-chat-message-row';
            row.innerHTML = '<div class="ds-chat-error"><i class="fas fa-exclamation-triangle"></i> ' + escapeHtml(msg.message || '未知错误') + '</div>';
            container.appendChild(row);
            currentAssistantEl = null;
            isChatStreaming = false;
            scrollChatToBottom();
        }
    }

    function createMessageRow(role, content) {
        const row = document.createElement('div');
        row.className = 'ds-chat-message-row ds-chat-message-row-' + role;
        const bubble = document.createElement('div');
        bubble.className = 'ds-chat-message ds-chat-message-' + role;
        const body = document.createElement('div');
        body.className = 'markdown-body';
        body.textContent = content;
        bubble.appendChild(body);
        row.appendChild(bubble);
        return row;
    }

    function scrollChatToBottom() {
        const c = document.getElementById('chat-messages');
        c.scrollTop = c.scrollHeight;
    }

    function sendChatMessage() {
        const input = document.getElementById('chat-input');
        const text = input.value.trim();
        if (!text || isChatStreaming) return;

        if (!chatWs || chatWs.readyState !== WebSocket.OPEN) connectChatWs();

        const container = document.getElementById('chat-messages');
        const empty = container.querySelector('.ds-empty-state');
        if (empty) empty.remove();

        const userRow = createMessageRow('user', '');
        const body = userRow.querySelector('.markdown-body');
        if (body) body.innerHTML = escapeHtml(text);
        container.appendChild(userRow);
        scrollChatToBottom();

        input.value = '';
        input.style.height = 'auto';
        isChatStreaming = true;
        currentAssistantEl = null;

        setTimeout(() => {
            if (chatWs && chatWs.readyState === WebSocket.OPEN) {
                chatWs.send(JSON.stringify({
                    provider_id: currentProviderId,
                    message: text
                }));
            }
        }, 100);
    }

    /* Chat input auto-resize & enter key */
    document.addEventListener('DOMContentLoaded', () => {
        const chatInput = document.getElementById('chat-input');
        chatInput.addEventListener('input', function() {
            this.style.height = 'auto';
            this.style.height = Math.min(this.scrollHeight, 150) + 'px';
        });
        chatInput.addEventListener('keydown', function(e) {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                sendChatMessage();
            }
        });
    });

    /* ===== Mirror ===== */
    let mirrorSps = null;
    let mirrorPps = null;
    let mirrorWidth = 0;
    let mirrorHeight = 0;
    let mirrorUseFallback = false;
    let mirrorFallbackTimer = null;

    async function startMirror() {
        document.getElementById('mirror-status').style.display = 'flex';
        document.getElementById('mirror-status-text').textContent = '正在启动...';
        mirrorSps = null; mirrorPps = null;
        mirrorWidth = 0; mirrorHeight = 0;
        mirrorUseFallback = false;

        // 获取设备分辨率
        try {
            const info = await apiGet('/api/device/info');
            if (info.ok && info.data && info.data.screen_size) {
                const m = info.data.screen_size.match(/(\d+)\s*x\s*(\d+)/);
                if (m) { mirrorWidth = parseInt(m[1]); mirrorHeight = parseInt(m[2]); }
            }
        } catch(e) {}
        if (!mirrorWidth) { mirrorWidth = 1080; mirrorHeight = 1920; }

        const res = await apiPost('/api/mirror/start', {});
        if (!res.ok) {
            showToast(res.message || '启动投屏失败', 'error');
            document.getElementById('mirror-status').style.display = 'none';
            return;
        }

        document.getElementById('mirror-start-btn').style.display = 'none';
        document.getElementById('mirror-stop-btn').style.display = '';
        document.getElementById('mirror-fullscreen-btn').style.display = '';
        document.getElementById('mirror-audio-btn').style.display = '';
        document.getElementById('mirror-controls').style.display = '';
        document.getElementById('mirror-placeholder').style.display = 'none';
        document.getElementById('mirror-canvas').style.display = 'block';
        document.getElementById('mirror-status-text').textContent = '连接中...';

        connectMirrorWs();
    }

    async function stopMirror() {
        if (mirrorWs) { mirrorWs.close(); mirrorWs = null; }
        if (mirrorDecoder) { try { mirrorDecoder.close(); } catch(e) {} mirrorDecoder = null; }
        if (mirrorFallbackTimer) { clearInterval(mirrorFallbackTimer); mirrorFallbackTimer = null; }
        stopMirrorAudio();
        // 退出全屏
        if (document.fullscreenElement) document.exitFullscreen();
        await apiPost('/api/mirror/stop', {});
        document.getElementById('mirror-start-btn').style.display = '';
        document.getElementById('mirror-stop-btn').style.display = 'none';
        document.getElementById('mirror-fullscreen-btn').style.display = 'none';
        document.getElementById('mirror-audio-btn').style.display = 'none';
        document.getElementById('mirror-controls').style.display = 'none';
        document.getElementById('mirror-placeholder').style.display = '';
        document.getElementById('mirror-canvas').style.display = 'none';
        document.getElementById('mirror-status').style.display = 'none';
        document.getElementById('mirror-recording').style.display = 'none';
        showToast('投屏已停止', 'info');
    }

    function connectMirrorWs() {
        if (mirrorWs && mirrorWs.readyState <= 1) return;
        const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
        mirrorWs = new WebSocket(proto + '//' + location.host + '/ws/mirror');
        mirrorWs.binaryType = 'arraybuffer';

        mirrorWs.onopen = () => {
            document.getElementById('mirror-status-text').textContent = '已连接';
            document.getElementById('mirror-status').style.display = 'none';
            if (!mirrorUseFallback) initMirrorDecoder();
        };

        mirrorWs.onmessage = (evt) => {
            if (evt.data instanceof ArrayBuffer) {
                if (!mirrorUseFallback) {
                    handleMirrorBinary(evt.data);
                }
            } else {
                try {
                    const msg = JSON.parse(evt.data);
                    if (msg.type === 'status') {
                        document.getElementById('mirror-status-text').textContent = msg.message || '';
                    }
                } catch (e) {}
            }
        };

        mirrorWs.onclose = () => {
            document.getElementById('mirror-status').style.display = 'flex';
            document.getElementById('mirror-status-text').textContent = '已断开';
        };

        mirrorWs.onerror = () => {
            showToast('投屏连接错误', 'error');
        };
    }

    /**
     * 解析后端二进制协议: [3字节tag][4字节大端长度][数据]
     * tag: "rst"=重置, "sps"=SPS, "pps"=PPS, "key"=IDR帧, "frm"=非IDR帧
     */
    function parseMirrorMessage(buffer) {
        const view = new DataView(buffer);
        if (buffer.byteLength < 7) return null;
        const tag = String.fromCharCode(view.getUint8(0), view.getUint8(1), view.getUint8(2));
        const len = view.getUint32(3, false);
        const data = new Uint8Array(buffer, 7, Math.min(len, buffer.byteLength - 7));
        return { tag, data };
    }

    /** Annex B NALU -> AVC 格式: 把起始码替换为4字节大端长度 */
    function naluToAvc(nalu) {
        // NALU 数据本身不含起始码（后端已剥离），直接加长度前缀
        const avc = new Uint8Array(4 + nalu.length);
        const view = new DataView(avc.buffer);
        view.setUint32(0, nalu.length, false);
        avc.set(nalu, 4);
        return avc;
    }

    /** 从 SPS 解析分辨率 */
    function parseSpsResolution(sps) {
        if (sps.length < 4) return null;
        // 简化解析: 从 SPS 的 pic_width/height_in_map_units 推算
        // 实际上用设备返回的分辨率更可靠，这里做备用
        return null;
    }

    /** 构建 avcC 配置 box */
    function buildAvcC(sps, pps) {
        const spsLen = sps.length;
        const ppsLen = pps.length;
        // avcC box: 5字节固定头 + 3字节SPS头 + sps + 1字节PPS头 + pps
        const buf = new Uint8Array(5 + 3 + spsLen + 1 + 2 + ppsLen);
        const v = new DataView(buf.buffer);
        buf[0] = sps[1]; // AVCProfileIndication
        buf[1] = sps[2]; // profile_compatibility
        buf[2] = sps[3]; // AVCLevelIndication
        buf[3] = 0xFF;   // lengthSizeMinusOne=3 (4字节长度)
        buf[4] = 0xE1;   // numOfSequenceParameterSets=1
        v.setUint16(5, spsLen, false);
        buf.set(sps, 7);
        const ppsOffset = 7 + spsLen;
        buf[ppsOffset] = 0x01; // numOfPictureParameterSets=1
        v.setUint16(ppsOffset + 1, ppsLen, false);
        buf.set(pps, ppsOffset + 3);
        return buf;
    }

    function initMirrorDecoder() {
        const canvas = document.getElementById('mirror-canvas');
        canvas.width = mirrorWidth;
        canvas.height = mirrorHeight;

        if (typeof VideoDecoder === 'undefined') {
            console.warn('VideoDecoder 不可用，切换到 JPEG fallback');
            startMirrorFallback();
            return;
        }

        try {
            if (mirrorDecoder) { try { mirrorDecoder.close(); } catch(e) {} }
            mirrorDecoder = new VideoDecoder({
                output: (frame) => {
                    const ctx = canvas.getContext('2d');
                    ctx.drawImage(frame, 0, 0, canvas.width, canvas.height);
                    frame.close();
                },
                error: (e) => {
                    console.error('VideoDecoder error:', e);
                    startMirrorFallback();
                }
            });
            // configure 会在收到 SPS/PPS 后调用
        } catch (e) {
            console.warn('VideoDecoder 初始化失败，切换到 JPEG fallback');
            startMirrorFallback();
        }
    }

    function handleMirrorBinary(buffer) {
        const msg = parseMirrorMessage(buffer);
        if (!msg) return;

        switch (msg.tag) {
            case 'rst':
                // 重置信号，清空缓存
                mirrorSps = null; mirrorPps = null;
                break;
            case 'sps':
                mirrorSps = msg.data;
                break;
            case 'pps':
                mirrorPps = msg.data;
                if (mirrorSps && mirrorDecoder) {
                    const avcC = buildAvcC(mirrorSps, mirrorPps);
                    try {
                        mirrorDecoder.configure({
                            codec: 'avc1.42001E',
                            codedWidth: mirrorWidth,
                            codedHeight: mirrorHeight,
                            description: avcC
                        });
                    } catch(e) {
                        console.error('VideoDecoder configure 失败:', e);
                        startMirrorFallback();
                    }
                }
                break;
            case 'key':
            case 'frm': {
                if (!mirrorDecoder || !mirrorSps) break;
                const avcData = naluToAvc(msg.data);
                try {
                    const chunk = new EncodedVideoChunk({
                        type: msg.tag === 'key' ? 'key' : 'delta',
                        timestamp: performance.now() * 1000,
                        data: avcData
                    });
                    mirrorDecoder.decode(chunk);
                } catch (e) {
                    // 解码失败，可能需要重新配置
                    mirrorSps = null; mirrorPps = null;
                }
                break;
            }
        }
    }

    /** JPEG Fallback: 通过 screencap 循环截图 */
    function startMirrorFallback() {
        mirrorUseFallback = true;
        if (mirrorDecoder) { try { mirrorDecoder.close(); } catch(e) {} mirrorDecoder = null; }
        const canvas = document.getElementById('mirror-canvas');
        const ctx = canvas.getContext('2d');
        canvas.width = mirrorWidth;
        canvas.height = mirrorHeight;

        async function fetchFrame() {
            if (!mirrorWs || mirrorWs.readyState > 1) return;
            try {
                const res = await apiGet('/api/mirror/screencap');
                if (res.ok && res.data) {
                    const blob = await (await fetch('data:image/jpeg;base64,' + res.data)).blob();
                    const url = URL.createObjectURL(blob);
                    const img = new Image();
                    img.onload = () => {
                        canvas.width = img.width;
                        canvas.height = img.height;
                        ctx.drawImage(img, 0, 0);
                        URL.revokeObjectURL(url);
                    };
                    img.src = url;
                }
            } catch(e) {}
        }

        mirrorFallbackTimer = setInterval(fetchFrame, 200); // ~5fps
        showToast('VideoDecoder 不可用，使用 JPEG 模式（约5fps）', 'warning');
    }

    /* Mirror touch/click control */
    document.addEventListener('DOMContentLoaded', () => {
        const canvas = document.getElementById('mirror-canvas');
        let touchStart = null;

        canvas.addEventListener('pointerdown', (e) => {
            const rect = canvas.getBoundingClientRect();
            touchStart = {
                x: Math.round((e.clientX - rect.left) / rect.width * mirrorWidth),
                y: Math.round((e.clientY - rect.top) / rect.height * mirrorHeight),
                time: Date.now()
            };
        });

        canvas.addEventListener('pointerup', (e) => {
            if (!touchStart) return;
            const rect = canvas.getBoundingClientRect();
            const x = Math.round((e.clientX - rect.left) / rect.width * mirrorWidth);
            const y = Math.round((e.clientY - rect.top) / rect.height * mirrorHeight);
            const dx = x - touchStart.x;
            const dy = y - touchStart.y;
            const dt = Date.now() - touchStart.time;

            if (Math.abs(dx) < 20 && Math.abs(dy) < 20 && dt < 300) {
                apiPost('/api/mirror/control', { action: 'touch', x: touchStart.x, y: touchStart.y });
            } else {
                apiPost('/api/mirror/control', { action: 'swipe', x: touchStart.x, y: touchStart.y, x2: x, y2: y, duration: dt });
            }
            touchStart = null;
        });

        // 鼠标滚轮 → 设备上下滑动
        canvas.addEventListener('wheel', (e) => {
            e.preventDefault();
            const rect = canvas.getBoundingClientRect();
            const cx = Math.round((e.clientX - rect.left) / rect.width * mirrorWidth);
            const cy = Math.round((e.clientY - rect.top) / rect.height * mirrorHeight);
            // 向上滚 = 屏幕上滑（内容下移），向下滚 = 屏幕下滑
            const dist = 300;
            const dy = e.deltaY > 0 ? -dist : dist; // deltaY>0=向下滚=屏幕上滑
            apiPost('/api/mirror/control', { action: 'swipe', x: cx, y: cy, x2: cx, y2: cy + dy, duration: 200 });
        }, { passive: false });
    });

    /* ===== Mirror Fullscreen ===== */
    function toggleMirrorFullscreen() {
        const container = document.getElementById('mirror-container');
        if (!document.fullscreenElement) {
            container.requestFullscreen().then(() => {
                document.getElementById('mirror-fullscreen-btn').innerHTML = '<i class="fas fa-compress"></i> 退出全屏';
                // 全屏时隐藏侧边栏
                document.querySelector('.side-tabs').style.display = 'none';
            }).catch(e => showToast('无法进入全屏: ' + e.message, 'error'));
        } else {
            document.exitFullscreen().then(() => {
                document.getElementById('mirror-fullscreen-btn').innerHTML = '<i class="fas fa-expand"></i> 全屏';
                document.querySelector('.side-tabs').style.display = '';
            });
        }
    }

    // 监听全屏变化（按Esc退出时恢复按钮文字）
    document.addEventListener('fullscreenchange', () => {
        if (!document.fullscreenElement) {
            document.getElementById('mirror-fullscreen-btn').innerHTML = '<i class="fas fa-expand"></i> 全屏';
            document.querySelector('.side-tabs').style.display = '';
        }
    });

    // 键盘快捷键（投屏激活时）
    document.addEventListener('keydown', (e) => {
        // 只在投屏运行中且不是输入框时响应
        if (currentTab !== 'mirror' || !mirrorWs || mirrorWs.readyState > 1) return;
        if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;

        switch (e.key) {
            case 'Escape':
                // 全屏时按Esc由浏览器处理，非全屏时按Esc = 返回
                if (!document.fullscreenElement) { e.preventDefault(); mirrorKey('BACK'); }
                break;
            case 'ArrowUp':
                e.preventDefault();
                mirrorVolumeUp();
                break;
            case 'ArrowDown':
                e.preventDefault();
                mirrorVolumeDown();
                break;
            case 'Home':
                e.preventDefault();
                mirrorKey('HOME');
                break;
            case 'F11':
                e.preventDefault();
                toggleMirrorFullscreen();
                break;
        }
    });

    /* ===== Mirror Device Controls ===== */
    const KEY_MAP = {
        'BACK': 4,
        'HOME': 3,
        'APP_SWITCH': 187,
        'MENU': 82,
        'POWER': 26,
        'VOLUME_UP': 24,
        'VOLUME_DOWN': 25
    };

    function mirrorKey(name) {
        const code = KEY_MAP[name];
        if (code !== undefined) {
            apiPost('/api/command', { command: 'input keyevent ' + code });
        }
    }

    function mirrorVolumeUp() {
        apiPost('/api/command', { command: 'input keyevent 24' });
    }

    function mirrorVolumeDown() {
        apiPost('/api/command', { command: 'input keyevent 25' });
    }

    /* ===== Mirror Audio (Web Audio API) ===== */
    let mirrorAudioWs = null;
    let mirrorAudioCtx = null;
    let mirrorAudioNode = null;
    let mirrorAudioEnabled = false;

    function toggleMirrorAudio() {
        if (mirrorAudioEnabled) {
            stopMirrorAudio();
        } else {
            startMirrorAudio();
        }
    }

    function startMirrorAudio() {
        try {
            // 创建 AudioContext: 48000Hz, 16-bit mono PCM
            const AudioCtx = window.AudioContext || window.webkitAudioContext;
            mirrorAudioCtx = new AudioCtx({ sampleRate: 48000 });

            // 使用 AudioWorklet 播放 PCM 数据
            const workletCode = `
                class PCMPlayerProcessor extends AudioWorkletProcessor {
                    constructor() {
                        super();
                        this._buffer = new Float32Array(0);
                        this.port.onmessage = (e) => {
                            const pcm16 = e.data; // Int16Array
                            const float32 = new Float32Array(pcm16.length);
                            for (let i = 0; i < pcm16.length; i++) {
                                float32[i] = pcm16[i] / 32768.0;
                            }
                            // 追加到缓冲区
                            const newBuf = new Float32Array(this._buffer.length + float32.length);
                            newBuf.set(this._buffer, 0);
                            newBuf.set(float32, this._buffer.length);
                            this._buffer = newBuf;
                        };
                    }
                    process(inputs, outputs) {
                        const output = outputs[0][0]; // mono
                        if (this._buffer.length >= output.length) {
                            output.set(this._buffer.subarray(0, output.length));
                            this._buffer = this._buffer.subarray(output.length);
                        } else {
                            // 缓冲不足时填充静音
                            output.fill(0);
                            if (this._buffer.length > 0) {
                                output.set(this._buffer, 0);
                                this._buffer = new Float32Array(0);
                            }
                        }
                        return true;
                    }
                }
                registerProcessor('pcm-player', PCMPlayerProcessor);
            `;
            const blob = new Blob([workletCode], { type: 'application/javascript' });
            const url = URL.createObjectURL(blob);

            mirrorAudioCtx.audioWorklet.addModule(url).then(() => {
                mirrorAudioNode = new AudioWorkletNode(mirrorAudioCtx, 'pcm-player');
                mirrorAudioNode.connect(mirrorAudioCtx.destination);
                URL.revokeObjectURL(url);

                // 连接音频 WebSocket
                const proto = location.protocol === 'https:' ? 'wss:' : 'ws:';
                mirrorAudioWs = new WebSocket(proto + '//' + location.host + '/ws/mirror/audio');
                mirrorAudioWs.binaryType = 'arraybuffer';

                mirrorAudioWs.onmessage = (evt) => {
                    if (evt.data instanceof ArrayBuffer && mirrorAudioNode) {
                        // 后端发送原始 PCM 16-bit mono 数据
                        const pcm16 = new Int16Array(evt.data);
                        mirrorAudioNode.port.postMessage(pcm16);
                    }
                };

                mirrorAudioWs.onclose = () => {
                    if (mirrorAudioEnabled) stopMirrorAudio();
                };

                mirrorAudioEnabled = true;
                const btn = document.getElementById('mirror-audio-btn');
                btn.innerHTML = '<i class="fas fa-volume-mute"></i> 静音';
                btn.classList.add('ds-btn-active');
                showToast('设备声音已开启', 'success');
            }).catch(e => {
                showToast('AudioWorklet 加载失败: ' + e.message, 'error');
            });
        } catch (e) {
            showToast('音频初始化失败: ' + e.message, 'error');
        }
    }

    function stopMirrorAudio() {
        mirrorAudioEnabled = false;
        if (mirrorAudioWs) { mirrorAudioWs.close(); mirrorAudioWs = null; }
        if (mirrorAudioNode) { mirrorAudioNode.disconnect(); mirrorAudioNode = null; }
        if (mirrorAudioCtx) { mirrorAudioCtx.close(); mirrorAudioCtx = null; }
        const btn = document.getElementById('mirror-audio-btn');
        btn.innerHTML = '<i class="fas fa-volume-up"></i> 声音';
        btn.classList.remove('ds-btn-active');
    }

    /* ===== Tasks ===== */
    async function loadTasks() {
        const res = await apiGet('/api/tasks');
        const list = document.getElementById('task-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><i class="fas fa-clipboard-list" style="font-size:2rem;opacity:0.3;"></i><p>暂无任务</p></div>';
            return;
        }
        list.innerHTML = res.data.map(t => {
            const name = escapeHtml(t.task_type || t.script || '未命名');
            const cmd = escapeHtml(t.script || '');
            const schedule = escapeHtml(t.time || '');
            const weeks = escapeHtml(t.weeks || '');
            const typeIcon = t.task_type === 'script' ? 'fa-scroll' : 'fa-terminal';
            const scheduleText = schedule ? (schedule + (weeks ? ' ' + weeks : '')) : '手动触发';
            return '<div class="script-item">' +
                '<div style="flex:1;min-width:0;">' +
                '<div style="display:flex;align-items:center;gap:8px;margin-bottom:6px;">' +
                '<i class="fas ' + typeIcon + '" style="opacity:0.5;font-size:14px;"></i>' +
                '<strong style="font-size:14px;">' + name + '</strong>' +
                '<span class="ds-badge" style="font-size:10px;">' + scheduleText + '</span>' +
                '</div>' +
                '<code style="font-size:12px;opacity:0.6;display:block;white-space:pre-wrap;word-break:break-all;max-height:60px;overflow:hidden;">' + cmd + '</code>' +
                '</div>' +
                '<div style="display:flex;gap:6px;flex-shrink:0;">' +
                '<button class="ds-btn-primary" style="font-size:12px;padding:6px 12px;" onclick="triggerTask(\'' + t.id + '\')" title="立即执行"><i class="fas fa-play"></i></button>' +
                '<button class="ds-btn-secondary" style="font-size:12px;padding:6px 12px;" onclick="editTask(\'' + t.id + '\')" title="编辑"><i class="fas fa-edit"></i></button>' +
                '<button class="ds-btn-danger" style="font-size:12px;padding:6px 12px;" onclick="deleteTask(\'' + t.id + '\')" title="删除"><i class="fas fa-trash"></i></button>' +
                '</div></div>';
        }).join('');
    }

    function openTaskModal() {
        document.getElementById('task-modal-title').textContent = '添加任务';
        document.getElementById('task-name').value = '';
        document.getElementById('task-command').value = '';
        document.getElementById('task-schedule').value = '';
        document.getElementById('task-type').value = 'custom';
        document.getElementById('task-edit-id').value = '';
        document.getElementById('task-modal').style.display = 'flex';
    }

    async function editTask(id) {
        const res = await apiGet('/api/tasks');
        if (!res.ok) return showToast('获取任务失败', 'error');
        const t = (res.data || []).find(x => x.id == id);
        if (!t) return showToast('任务不存在', 'error');
        document.getElementById('task-modal-title').textContent = '编辑任务';
        document.getElementById('task-name').value = t.task_type || '';
        document.getElementById('task-command').value = t.script || '';
        document.getElementById('task-schedule').value = t.time || '';
        document.getElementById('task-type').value = 'custom';
        document.getElementById('task-edit-id').value = id;
        document.getElementById('task-modal').style.display = 'flex';
    }

    async function saveTask() {
        const name = document.getElementById('task-name').value.trim();
        const command = document.getElementById('task-command').value.trim();
        const schedule = document.getElementById('task-schedule').value.trim();
        const taskType = document.getElementById('task-type').value;
        if (!command) return showToast('命令为必填', 'error');

        const editId = document.getElementById('task-edit-id').value;
        if (editId) {
            await apiDelete('/api/tasks/' + editId);
        }
        const res = await apiPost('/api/tasks', { 
            time: schedule || '* * * * *', 
            script: command, 
            task_type: name || taskType || 'custom'
        });
        if (res.ok) {
            showToast('任务已保存', 'success');
            document.getElementById('task-modal').style.display = 'none';
            loadTasks();
        } else {
            showToast(res.message || '保存失败', 'error');
        }
    }

    async function deleteTask(id) {
        if (!confirm('确定删除此任务？')) return;
        const res = await apiDelete('/api/tasks/' + id);
        if (res.ok) { showToast('已删除', 'success'); loadTasks(); }
        else showToast(res.message || '删除失败', 'error');
    }

    async function triggerTask(id) {
        const res = await apiGet('/api/tasks');
        if (!res.ok) return showToast('获取任务失败', 'error');
        const task = (res.data || []).find(t => t.id == id);
        if (!task) return showToast('任务不存在', 'error');
        const triggerRes = await apiPost('/api/trigger', { script: task.script });
        if (triggerRes.ok) showToast('任务已触发', 'success');
        else showToast(triggerRes.message || '触发失败', 'error');
    }

    /* ===== Scripts ===== */
    async function loadScripts() {
        const res = await apiGet('/api/scripts');
        const list = document.getElementById('script-list');
        if (!res.ok || !res.data || res.data.length === 0) {
            list.innerHTML = '<div class="ds-empty-state"><i class="fas fa-code" style="font-size:2rem;opacity:0.3;"></i><p>暂无脚本</p></div>';
            return;
        }
        list.innerHTML = res.data.map(s => {
            const name = typeof s === 'string' ? s : s.name;
            return '<div class="script-item">' +
                '<div style="flex:1;min-width:0;">' +
                '<div style="display:flex;align-items:center;gap:8px;">' +
                '<i class="fas fa-file-code" style="opacity:0.5;font-size:14px;"></i>' +
                '<strong style="font-size:14px;">' + escapeHtml(name) + '</strong>' +
                '<span class="ds-badge" style="font-size:10px;">Shell</span>' +
                '</div>' +
                '</div>' +
                '<div style="display:flex;gap:6px;flex-shrink:0;">' +
                '<button class="ds-btn-primary" style="font-size:12px;padding:6px 12px;" onclick="runScript(\'' + escapeHtml(name) + '\')" title="运行"><i class="fas fa-play"></i> 运行</button>' +
                '<button class="ds-btn-secondary" style="font-size:12px;padding:6px 12px;" onclick="openScriptEditor(\'' + escapeHtml(name) + '\')" title="编辑"><i class="fas fa-edit"></i> 编辑</button>' +
                '<button class="ds-btn-danger" style="font-size:12px;padding:6px 12px;" onclick="deleteScript(\'' + escapeHtml(name) + '\')" title="删除"><i class="fas fa-trash"></i></button>' +
                '</div></div>';
        }).join('');
    }

    function openNewScriptModal() {
        document.getElementById('script-modal-title').textContent = '新建脚本';
        document.getElementById('script-editor').value = '#!/bin/bash\n# TaskMod 脚本\n\n';
        document.getElementById('script-modal').style.display = 'flex';
        document.getElementById('script-modal').dataset.name = '';
        document.getElementById('script-modal').dataset.isNew = 'true';
        document.getElementById('script-editor').focus();
        updateLineInfo();
    }

    async function runScript(name) {
        const res = await apiPost('/api/trigger', { script: name });
        if (res.ok) showToast('脚本已触发: ' + name, 'success');
        else showToast(res.message || '运行失败', 'error');
    }

    async function openScriptEditor(name) {
        document.getElementById('script-modal-title').textContent = '编辑脚本: ' + name;
        document.getElementById('script-editor').value = '加载中...';
        document.getElementById('script-modal').style.display = 'flex';
        document.getElementById('script-modal').dataset.name = name;
        document.getElementById('script-modal').dataset.isNew = 'false';
        const res = await apiGet('/api/scripts/' + encodeURIComponent(name));
        if (res.ok) {
            document.getElementById('script-editor').value = typeof res.data === 'string' ? res.data : (res.data?.content || JSON.stringify(res.data, null, 2));
        } else {
            document.getElementById('script-editor').value = '#!/bin/bash\n';
        }
        updateLineInfo();
    }

    async function saveScript() {
        const modal = document.getElementById('script-modal');
        let name = modal.dataset.name;
        const isNew = modal.dataset.isNew === 'true';

        if (isNew && !name) {
            name = prompt('请输入脚本文件名 (以 .sh 结尾):', 'script_' + Date.now() + '.sh');
            if (!name) return;
            if (!name.endsWith('.sh')) name += '.sh';
            modal.dataset.name = name;
        }
        if (!name) return showToast('未知脚本名', 'error');

        const content = document.getElementById('script-editor').value;
        const res = await apiPut('/api/scripts/' + encodeURIComponent(name), { content });
        if (res.ok) {
            showToast('脚本已保存', 'success');
            modal.style.display = 'none';
            loadScripts();
        } else {
            showToast(res.message || '保存失败', 'error');
        }
    }

    async function deleteScript(name) {
        if (!confirm('确定删除脚本 "' + name + '"？')) return;
        const res = await apiDelete('/api/scripts/' + encodeURIComponent(name));
        if (res.ok) { showToast('已删除', 'success'); loadScripts(); }
        else showToast(res.message || '删除失败', 'error');
    }

    // 脚本编辑器增强：Tab缩进、Ctrl+S保存、行号显示
    document.addEventListener('DOMContentLoaded', () => {
        const editor = document.getElementById('script-editor');
        if (!editor) return;

        // Tab 键缩进
        editor.addEventListener('keydown', (e) => {
            if (e.key === 'Tab') {
                e.preventDefault();
                const start = editor.selectionStart;
                const end = editor.selectionEnd;
                if (e.shiftKey) {
                    // Shift+Tab: 取消缩进
                    const lineStart = editor.value.lastIndexOf('\n', start - 1) + 1;
                    const lineText = editor.value.substring(lineStart, start);
                    if (lineText.startsWith('  ')) {
                        editor.value = editor.value.substring(0, lineStart) + editor.value.substring(lineStart + 2);
                        editor.selectionStart = editor.selectionEnd = start - 2;
                    } else if (lineText.startsWith('\t')) {
                        editor.value = editor.value.substring(0, lineStart) + editor.value.substring(lineStart + 1);
                        editor.selectionStart = editor.selectionEnd = start - 1;
                    }
                } else {
                    // Tab: 插入2个空格
                    editor.value = editor.value.substring(0, start) + '  ' + editor.value.substring(end);
                    editor.selectionStart = editor.selectionEnd = start + 2;
                }
                updateLineInfo();
            }

            // Ctrl+S 保存
            if (e.key === 's' && (e.ctrlKey || e.metaKey)) {
                e.preventDefault();
                saveScript();
            }
        });

        // 光标位置更新
        editor.addEventListener('click', updateLineInfo);
        editor.addEventListener('keyup', updateLineInfo);
        editor.addEventListener('input', updateLineInfo);
    });

    function updateLineInfo() {
        const editor = document.getElementById('script-editor');
        if (!editor) return;
        const text = editor.value.substring(0, editor.selectionStart);
        const lines = text.split('\n');
        const line = lines.length;
        const col = lines[lines.length - 1].length + 1;
        const info = document.getElementById('script-line-info');
        if (info) info.textContent = '行 ' + line + ', 列 ' + col;
    }

    function formatScript() {
        const editor = document.getElementById('script-editor');
        if (!editor) return;
        // 简单格式化：去除行尾空格，确保末尾有换行
        editor.value = editor.value.replace(/[ \t]+$/gm, '').replace(/\n*$/, '\n');
        showToast('已格式化', 'info');
    }

    /* ===== TTS ===== */
    async function loadTtsEngines() {
        const res = await apiGet('/api/tts/engines');
        const sel = document.getElementById('tts-engine');
        if (res.ok && res.data && res.data.length > 0) {
            sel.innerHTML = res.data.map(e => {
                const val = typeof e === 'string' ? e : (e.package_name || e.id || '');
                const label = typeof e === 'string' ? e : (e.label || e.name || val);
                return '<option value="' + escapeHtml(val) + '">' + escapeHtml(label) + '</option>';
            }).join('');
        } else {
            sel.innerHTML = '<option value="">无可用引擎</option>';
        }
    }

    async function speakTTS() {
        const text = document.getElementById('tts-text').value.trim();
        if (!text) return showToast('请输入文本', 'error');
        const engine = document.getElementById('tts-engine').value;
        if (!engine) return showToast('请先选择语音引擎', 'error');
        const res = await apiPost('/api/tts/speak', {
            text,
            engine,
            rate: parseFloat(document.getElementById('tts-speed').value),
            pitch: parseFloat(document.getElementById('tts-pitch').value),
            volume: parseFloat(document.getElementById('tts-volume').value)
        });
        if (res.ok) showToast(res.message || '朗读中...', 'success');
        else showToast(res.message || '朗读失败', 'error');
    }

    async function stopTTS() {
        const res = await apiPost('/api/tts/stop', {});
        if (res.ok) showToast('已停止', 'info');
        else showToast(res.message || '停止失败', 'error');
    }

    async function loadTtsSettings() {
        const res = await apiGet('/api/tts/settings');
        if (res.ok && res.data) {
            const d = res.data;
            if (d.default_engine) {
                const sel = document.getElementById('tts-engine');
                if (sel.querySelector('option[value="' + d.default_engine + '"]')) {
                    sel.value = d.default_engine;
                }
            }
            if (d.default_rate != null) {
                document.getElementById('tts-speed').value = d.default_rate;
                document.getElementById('tts-speed-val').textContent = d.default_rate;
            }
            if (d.default_pitch != null) {
                document.getElementById('tts-pitch').value = d.default_pitch;
                document.getElementById('tts-pitch-val').textContent = d.default_pitch;
            }
            if (d.default_volume != null) {
                document.getElementById('tts-volume').value = d.default_volume;
                document.getElementById('tts-volume-val').textContent = d.default_volume;
            }
            document.getElementById('tts-replace-enabled').checked = !!d.replace_enabled;
            document.getElementById('tts-split-enabled').checked = d.split_enabled !== false;
            // 加载引擎参数列表
            renderEngineParams(d.engine_params || []);
            // 加载替换规则列表
            loadReplaceRules();
        }
    }

    async function testTTS() {
        const engine = document.getElementById('tts-engine').value;
        const res = await apiPost('/api/tts/test', {
            text: '这是一段测试语音，TaskMod TTS功能正常。',
            engine: engine || undefined
        });
        if (res.ok) showToast('测试语音播放中...', 'success');
        else showToast(res.message || '测试失败', 'error');
    }

    /* 试听：用当前选中引擎 + 默认参数播放固定文本 */
    async function auditionTTS() {
        const engine = document.getElementById('tts-engine').value;
        if (!engine) return showToast('请先选择语音引擎', 'error');
        const res = await apiPost('/api/tts/test', {
            text: '你好，这是' + engine.split('.').pop() + '引擎的试听测试。',
            engine
        });
        if (res.ok) showToast('试听中...', 'success');
        else showToast(res.message || '试听失败', 'error');
    }

    async function saveTtsSettings() {
        const body = {
            rate: parseFloat(document.getElementById('tts-speed').value),
            pitch: parseFloat(document.getElementById('tts-pitch').value),
            volume: parseFloat(document.getElementById('tts-volume').value),
            replace_enabled: document.getElementById('tts-replace-enabled').checked,
            split_enabled: document.getElementById('tts-split-enabled').checked
        };
        const res = await apiPut('/api/tts/settings', body);
        if (res.ok) showToast('TTS设置已保存', 'success');
        else showToast(res.message || '保存失败', 'error');
    }

    /* ===== 按引擎独立参数 ===== */
    function renderEngineParams(list) {
        const el = document.getElementById('engine-params-list');
        if (!list || list.length === 0) {
            el.innerHTML = '<div style="color:#999;padding:8px 0;">暂无引擎独立参数，点击"添加"为特定引擎配置参数</div>';
            return;
        }
        el.innerHTML = list.map(p =>
            '<div style="display:flex;align-items:center;gap:8px;padding:6px 0;border-bottom:1px solid #f0f0f0;">' +
            '<span style="flex:1;font-family:monospace;font-size:12px;">' + escapeHtml(p.engine) + '</span>' +
            '<span style="font-size:12px;color:#666;">语速:' + p.rate + ' 音调:' + p.pitch + ' 音量:' + p.volume + '</span>' +
            '<button class="ds-btn-danger" style="padding:2px 8px;font-size:11px;" onclick="deleteEngineParam(\'' + escapeHtml(p.engine) + '\')"><i class="fas fa-trash"></i></button>' +
            '</div>'
        ).join('');
    }

    function showAddEngineParamDialog() {
        const engine = prompt('输入引擎包名 (如 com.iflytek.tts):');
        if (!engine) return;
        const rate = parseFloat(prompt('语速 (默认1.0):', '1.0')) || 1.0;
        const pitch = parseFloat(prompt('音调 (默认1.0):', '1.0')) || 1.0;
        const volume = parseFloat(prompt('音量 (默认1.0):', '1.0')) || 1.0;
        apiPost('/api/tts/engine-params', { engine, rate, pitch, volume }).then(res => {
            if (res.ok) { showToast('已保存', 'success'); loadTtsSettings(); }
            else showToast(res.message || '保存失败', 'error');
        });
    }

    async function deleteEngineParam(engine) {
        if (!confirm('删除引擎 ' + engine + ' 的独立参数？')) return;
        const res = await apiDelete('/api/tts/engine-params/' + encodeURIComponent(engine));
        if (res.ok) { showToast('已删除', 'info'); loadTtsSettings(); }
        else showToast(res.message || '删除失败', 'error');
    }

    /* ===== 文本替换规则 ===== */
    async function loadReplaceRules() {
        const res = await apiGet('/api/tts/replace-rules');
        renderReplaceRules(res.ok ? res.data : []);
    }

    function renderReplaceRules(list) {
        const el = document.getElementById('replace-rules-list');
        if (!list || list.length === 0) {
            el.innerHTML = '<div style="color:#999;padding:8px 0;">暂无替换规则，点击"添加规则"创建</div>';
            return;
        }
        list.sort((a, b) => a.order - b.order);
        el.innerHTML = list.map(r =>
            '<div style="display:flex;align-items:center;gap:8px;padding:6px 0;border-bottom:1px solid #f0f0f0;">' +
            '<input type="checkbox" ' + (r.enabled ? 'checked' : '') + ' onchange="toggleReplaceRule(\'' + r.id + '\',this.checked)" title="启用/禁用">' +
            '<span style="flex:1;font-size:12px;">' + escapeHtml(r.name) + '</span>' +
            '<span style="font-size:11px;color:#888;font-family:monospace;">' + escapeHtml(r.pattern) + ' → ' + escapeHtml(r.replacement) + '</span>' +
            '<span style="font-size:10px;color:#aaa;">' + (r.is_regex ? '正则' : '文本') + '</span>' +
            '<button class="ds-btn-danger" style="padding:2px 8px;font-size:11px;" onclick="deleteReplaceRule(\'' + r.id + '\')"><i class="fas fa-trash"></i></button>' +
            '</div>'
        ).join('');
    }

    function showAddReplaceRuleDialog() {
        const name = prompt('规则名称:');
        if (!name) return;
        const pattern = prompt('匹配文本 (支持正则):');
        if (pattern === null) return;
        const replacement = prompt('替换为:');
        if (replacement === null) return;
        const isRegex = confirm('是否使用正则表达式？\n确定=正则，取消=纯文本');
        const id = 'rule_' + Date.now();
        apiPost('/api/tts/replace-rules', { id, name, pattern, replacement, is_regex: isRegex, enabled: true, order: 0 }).then(res => {
            if (res.ok) { showToast('已添加', 'success'); loadReplaceRules(); }
            else showToast(res.message || '添加失败', 'error');
        });
    }

    async function toggleReplaceRule(id, enabled) {
        // 简单实现：获取规则列表，修改enabled，PUT回去
        const res = await apiGet('/api/tts/replace-rules');
        if (!res.ok) return;
        const rule = res.data.find(r => r.id === id);
        if (!rule) return;
        rule.enabled = enabled;
        await apiPut('/api/tts/replace-rules/' + id, rule);
    }

    async function deleteReplaceRule(id) {
        if (!confirm('删除此替换规则？')) return;
        const res = await apiDelete('/api/tts/replace-rules/' + id);
        if (res.ok) { showToast('已删除', 'info'); loadReplaceRules(); }
        else showToast(res.message || '删除失败', 'error');
    }

    /* ===== 语音设置（朗读回复 + 语音输入） ===== */
    const VOICE_STORAGE_KEY = 'taskmod_voice_settings';
    let voiceSettings = { inputEnabled: false, readAloudEnabled: false, rate: 1.0, pitch: 1.0 };
    let voiceRecognition = null;
    let isVoiceListening = false;

    function loadVoiceSettings() {
        try {
            const saved = JSON.parse(localStorage.getItem(VOICE_STORAGE_KEY));
            if (saved && typeof saved === 'object') {
                voiceSettings = { ...voiceSettings, ...saved };
            }
        } catch (e) { /* ignore */ }
        // 同步到UI
        const inputEl = document.getElementById('voice-input-enabled');
        const readEl = document.getElementById('voice-read-aloud');
        const rateEl = document.getElementById('voice-rate');
        const pitchEl = document.getElementById('voice-pitch');
        if (inputEl) inputEl.checked = voiceSettings.inputEnabled;
        if (readEl) readEl.checked = voiceSettings.readAloudEnabled;
        if (rateEl) { rateEl.value = voiceSettings.rate; document.getElementById('voice-rate-val').textContent = voiceSettings.rate; }
        if (pitchEl) { pitchEl.value = voiceSettings.pitch; document.getElementById('voice-pitch-val').textContent = voiceSettings.pitch; }
        // 同步朗读回复按钮状态
        ttsReplyEnabled = voiceSettings.readAloudEnabled;
        updateTtsReplyBtn();
        // 显示/隐藏语音输入按钮
        updateVoiceInputBtn();
    }

    function saveVoiceSettings() {
        voiceSettings.inputEnabled = document.getElementById('voice-input-enabled')?.checked || false;
        voiceSettings.readAloudEnabled = document.getElementById('voice-read-aloud')?.checked || false;
        voiceSettings.rate = parseFloat(document.getElementById('voice-rate')?.value || '1.0');
        voiceSettings.pitch = parseFloat(document.getElementById('voice-pitch')?.value || '1.0');
        localStorage.setItem(VOICE_STORAGE_KEY, JSON.stringify(voiceSettings));
        // 同步朗读回复开关
        ttsReplyEnabled = voiceSettings.readAloudEnabled;
        updateTtsReplyBtn();
        updateVoiceInputBtn();
    }

    function updateVoiceInputBtn() {
        const btn = document.getElementById('voice-input-btn');
        if (!btn) return;
        // 检查浏览器是否支持语音识别
        const hasSpeechRecognition = !!(window.SpeechRecognition || window.webkitSpeechRecognition);
        btn.style.display = (voiceSettings.inputEnabled && hasSpeechRecognition) ? '' : 'none';
    }

    /* 语音输入 */
    function toggleVoiceInput() {
        if (isVoiceListening) {
            stopVoiceInput();
        } else {
            startVoiceInput();
        }
    }

    function startVoiceInput() {
        const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
        if (!SpeechRecognition) {
            showToast('当前浏览器不支持语音识别', 'error');
            return;
        }
        const recognition = new SpeechRecognition();
        recognition.lang = navigator.language || 'zh-CN';
        recognition.interimResults = true;
        recognition.continuous = false;

        recognition.onresult = function(event) {
            const transcript = Array.from(event.results)
                .map(function(result) { return result[0] ? result[0].transcript : ''; })
                .join('')
                .trim();
            if (transcript) {
                document.getElementById('chat-input').value = transcript;
            }
        };
        recognition.onstart = function() {
            isVoiceListening = true;
            const btn = document.getElementById('voice-input-btn');
            if (btn) { btn.style.color = '#ef4444'; btn.title = '停止语音输入'; }
            showToast('正在听...', 'info');
        };
        recognition.onend = function() {
            isVoiceListening = false;
            const btn = document.getElementById('voice-input-btn');
            if (btn) { btn.style.color = '#666'; btn.title = '语音输入'; }
        };
        recognition.onerror = function(e) {
            isVoiceListening = false;
            const btn = document.getElementById('voice-input-btn');
            if (btn) { btn.style.color = '#666'; btn.title = '语音输入'; }
            if (e.error !== 'no-speech') {
                showToast('语音识别错误: ' + e.error, 'error');
            }
        };
        voiceRecognition = recognition;
        recognition.start();
    }

    function stopVoiceInput() {
        if (voiceRecognition) {
            voiceRecognition.stop();
            voiceRecognition = null;
        }
        isVoiceListening = false;
    }

    /* 通用TTS朗读函数，供AI对话调用 */
    async function ttsReadText(text) {
        if (!text || !text.trim()) return;
        const maxLen = 2000;
        const speakText = text.length > maxLen ? text.substring(0, maxLen) + '...(已截断)' : text;
        const engine = document.getElementById('tts-engine')?.value;
        if (!engine) {
            showToast('请先在TTS页面选择引擎', 'error');
            return;
        }
        try {
            const res = await apiPost('/api/tts/speak', {
                text: speakText,
                engine,
                rate: voiceSettings.rate != null ? voiceSettings.rate : parseFloat(document.getElementById('tts-speed')?.value || '1.0'),
                 pitch: voiceSettings.pitch != null ? voiceSettings.pitch : parseFloat(document.getElementById('tts-pitch')?.value || '1.0'),
                volume: parseFloat(document.getElementById('tts-volume')?.value || '1.0')
            });
            if (!res.ok) showToast('TTS: ' + (res.message || '朗读失败'), 'error');
        } catch (e) {
            showToast('TTS请求失败: ' + e.message, 'error');
        }
    }

    /* 切换朗读AI回复开关 */
    function toggleTtsReply() {
        ttsReplyEnabled = !ttsReplyEnabled;
        voiceSettings.readAloudEnabled = ttsReplyEnabled;
        localStorage.setItem(VOICE_STORAGE_KEY, JSON.stringify(voiceSettings));
        // 同步配置页面的开关
        const readEl = document.getElementById('voice-read-aloud');
        if (readEl) readEl.checked = ttsReplyEnabled;
        updateTtsReplyBtn();
        showToast(ttsReplyEnabled ? 'AI回复朗读已开启' : 'AI回复朗读已关闭', ttsReplyEnabled ? 'success' : 'info');
    }

    function updateTtsReplyBtn() {
        const btn = document.getElementById('tts-reply-toggle');
        if (!btn) return;
        if (ttsReplyEnabled) {
            btn.style.background = '#10b981';
            btn.style.color = '#fff';
            btn.title = '朗读AI回复: 已开启';
        } else {
            btn.style.background = '';
            btn.style.color = '';
            btn.title = '朗读AI回复: 已关闭';
        }
    }

    /* 页面加载时恢复语音设置 */
    (function() {
        loadVoiceSettings();
        document.addEventListener('DOMContentLoaded', function() {
            loadVoiceSettings();
        });
    })();

    /* ===== Config ===== */
    async function loadEmailConfig() {
        const res = await apiGet('/api/email/config');
        if (res.ok && res.data) {
            const d = res.data;
            document.getElementById('cfg-smtp-host').value = d.smtp_server || d.host || '';
            document.getElementById('cfg-smtp-port').value = d.smtp_port || d.port || '';
            document.getElementById('cfg-smtp-user').value = d.username || d.user || '';
            document.getElementById('cfg-smtp-to').value = d.to || '';
        }
    }

    async function saveEmailConfig() {
        const body = {
            enable_notify: 'true',
            smtp_server: document.getElementById('cfg-smtp-host').value.trim(),
            smtp_port: parseInt(document.getElementById('cfg-smtp-port').value) || 587,
            username: document.getElementById('cfg-smtp-user').value.trim(),
            password: document.getElementById('cfg-smtp-pass').value.trim(),
            from: document.getElementById('cfg-smtp-user').value.trim(),
            to: document.getElementById('cfg-smtp-to').value.trim(),
            subject: 'TaskMod 通知',
            body: ''
        };
        const res = await apiPut('/api/email/config', body);
        if (res.ok) showToast('邮件配置已保存', 'success');
        else showToast(res.message || '保存失败', 'error');
    }

    async function testSendEmail() {
        const res = await apiPost('/api/send-email', {
            subject: 'TaskMod 测试邮件',
            body: '这是一封来自 TaskMod 的测试邮件。'
        });
        if (res.ok) showToast('测试邮件已发送', 'success');
        else showToast(res.message || '发送失败', 'error');
    }

    async function loadMqttConfig() {
        const res = await apiGet('/api/mqtt/config');
        if (res.ok && res.data) {
            const d = res.data;
            document.getElementById('cfg-mqtt-broker').value = d.broker || '';
            document.getElementById('cfg-mqtt-topic').value = d.topic || '';
            document.getElementById('cfg-mqtt-clientid').value = d.client_id || '';
            document.getElementById('cfg-mqtt-user').value = d.user || '';
        }
    }

    async function saveMqttConfig() {
        const body = {
            broker: document.getElementById('cfg-mqtt-broker').value.trim(),
            topic: document.getElementById('cfg-mqtt-topic').value.trim(),
            client_id: document.getElementById('cfg-mqtt-clientid').value.trim(),
            user: document.getElementById('cfg-mqtt-user').value.trim(),
            password: document.getElementById('cfg-mqtt-pass').value.trim()
        };
        const res = await apiPut('/api/mqtt/config', body);
        if (res.ok) showToast('MQTT 配置已保存', 'success');
        else showToast(res.message || '保存失败', 'error');
    }

    async function execCommand() {
        const cmd = document.getElementById('cfg-command').value.trim();
        if (!cmd) return showToast('请输入命令', 'error');
        const output = document.getElementById('cfg-command-output');
        output.textContent = '执行中...';
        const res = await apiPost('/api/command', { command: cmd });
        if (res.ok) {
            output.textContent = res.data?.output || res.data?.stdout || JSON.stringify(res.data, null, 2) || '(无输出)';
        } else {
            output.textContent = '错误: ' + (res.message || '执行失败');
        }
    }

    /* ===== Logs ===== */
    async function loadLogs() {
        const res = await apiGet('/api/logs');
        const viewer = document.getElementById('log-viewer');
        if (!res.ok || !res.data || (Array.isArray(res.data) && res.data.length === 0)) {
            viewer.innerHTML = '<div class="ds-empty-state"><i class="fas fa-scroll" style="font-size:2rem;opacity:0.3;"></i><p>暂无日志</p></div>';
            return;
        }
        const logs = Array.isArray(res.data) ? res.data : [res.data];
        viewer.innerHTML = logs.map(l => {
            const text = typeof l === 'string' ? l : (l.message || l.text || JSON.stringify(l));
            const time = l.timestamp || l.time || '';
            return '<div class="log-line">' + (time ? '<span class="log-time">' + escapeHtml(time) + '</span> ' : '') + escapeHtml(text) + '</div>';
        }).join('');
        viewer.scrollTop = viewer.scrollHeight;
    }

    async function clearLogs() {
        if (!confirm('确定清除所有日志？')) return;
        const res = await apiPost('/api/logs/clear', {});
        if (res.ok) { showToast('日志已清除', 'success'); loadLogs(); }
        else showToast(res.message || '清除失败', 'error');
    }

    function toggleLogAutoRefresh() {
        const cb = document.getElementById('log-autorefresh-cb');
        cb.checked = !cb.checked;
        if (cb.checked) {
            logAutoRefreshInterval = setInterval(loadLogs, 3000);
            showToast('自动刷新已开启', 'info');
        } else {
            if (logAutoRefreshInterval) clearInterval(logAutoRefreshInterval);
            logAutoRefreshInterval = null;
            showToast('自动刷新已关闭', 'info');
        }
    }

    /* ===== Init ===== */
    document.addEventListener('DOMContentLoaded', () => {
        refreshStatus();
    });