// MobSF-Win frontend. Uses the globally-exposed Tauri API
// (app.withGlobalTauri = true) so no npm package is required.
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

const $ = (id) => document.getElementById(id);
const pathInput = $('path');
const outdirInput = $('outdir');
const toolSelect = $('tool');
const resultEl = $('result');
const statusEl = $('status');

function escapeHtml(s) {
  return String(s)
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

function setStatus(msg, kind) {
  statusEl.textContent = msg || '';
  statusEl.className = 'status' + (kind ? ' ' + kind : '');
}

function kv(label, value) {
  if (value === undefined || value === null || value === '') return '';
  return `<div class="kv"><span class="k">${escapeHtml(label)}</span><span class="v">${escapeHtml(value)}</span></div>`;
}

function componentList(title, comps) {
  if (!comps.length) return '';
  const rows = comps
    .map((c) => {
      const tags = [];
      if (c.exported !== null && c.exported !== undefined)
        tags.push(`<span class="tag ${c.exported ? 'warn' : 'ok'}">exported=${c.exported}</span>`);
      if (c.enabled !== null && c.enabled !== undefined)
        tags.push(`<span class="tag">enabled=${c.enabled}</span>`);
      if (c.permission) tags.push(`<span class="tag perm">perm: ${escapeHtml(c.permission)}</span>`);
      return `<li><code>${escapeHtml(c.name)}</code> ${tags.join(' ')}</li>`;
    })
    .join('');
  return `<div class="block"><h3>${escapeHtml(title)} (${comps.length})</h3><ul class="comps">${rows}</ul></div>`;
}

function render(m) {
  const sdk = `min=${m.min_sdk_version ?? '-'} target=${m.target_sdk_version ?? '-'} max=${m.max_sdk_version ?? '-'} compile=${m.compile_sdk_version ?? '-'}`;
  const perms = m.permissions.length
    ? `<div class="block"><h3>权限 (${m.permissions.length})</h3><ul class="comps">${m.permissions
        .map((p) => `<li><code>${escapeHtml(p.name)}</code>${p.max_sdk_version ? ` <span class="tag">maxSdk=${p.max_sdk_version}</span>` : ''}</li>`)
        .join('')}</ul></div>`
    : '<div class="block"><h3>权限</h3><p class="muted">无</p></div>';

  resultEl.innerHTML = `
    <div class="block">
      <h3>应用标识</h3>
      ${kv('包名', m.package)}
      ${kv('versionCode', m.version_code)}
      ${kv('versionName', m.version_name)}
      ${kv('SDK', sdk)}
    </div>
    ${perms}
    ${componentList('Activities', m.activities)}
    ${componentList('Services', m.services)}
    ${componentList('Receivers', m.receivers)}
    ${componentList('Providers', m.providers)}
  `;
  resultEl.classList.remove('hidden');
}

async function analyze() {
  const p = pathInput.value.trim();
  if (!p) {
    setStatus('请先提供 APK 路径。', 'err');
    return;
  }
  setStatus('正在解析 AndroidManifest.xml …');
  try {
    const m = await invoke('analyze_apk', { path: p });
    render(m);
    setStatus('解析完成（纯 Rust，无需外部工具）。', 'ok');
  } catch (e) {
    setStatus('分析失败：' + e, 'err');
    resultEl.classList.add('hidden');
  }
}

async function decompile() {
  const p = pathInput.value.trim();
  if (!p) {
    setStatus('请先提供 APK 路径。', 'err');
    return;
  }
  const out = outdirInput.value.trim() || (p + '.out');
  const tool = toolSelect.value;
  setStatus(`正在用 ${tool} 反编译 …（需要外部工具）`);
  try {
    const r = await invoke('decompile_apk', { path: p, tool, outDir: out });
    if (r.success) {
      setStatus(`反编译完成，输出目录：${r.output_dir}`, 'ok');
    } else {
      setStatus('反编译返回非零退出码，请查看 stderr：' + r.stderr.slice(0, 500), 'err');
    }
  } catch (e) {
    setStatus('反编译失败：' + e, 'err');
  }
}

$('analyze').addEventListener('click', analyze);
$('decompile').addEventListener('click', decompile);

listen('tauri://drag-drop', (e) => {
  const paths = (e.payload && e.payload.paths) || [];
  if (paths.length) {
    pathInput.value = paths[0];
    analyze();
  }
});
