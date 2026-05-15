const REPO    = '0xarchit/pauseCat';
const API     = 'https://api.github.com/repos/' + REPO;
const CACHE   = 'pausecat_stats_v1';
const TTL     = 5 * 60 * 1000; // 5 minutes

const FALLBACK = {
  stars:     '—',
  downloads: '—',
  version:   'v1.1.2',
  msiUrl:    `https://github.com/${REPO}/releases/latest`,
};

function fmt(n) {
  if (typeof n !== 'number') return '—';
  return n >= 1000 ? (n / 1000).toFixed(1).replace(/\.0$/, '') + 'k' : String(n);
}

function applyStats(s) {
  document.querySelectorAll('[data-stat="stars"]')
    .forEach(el => { el.textContent = s.stars; });
  document.querySelectorAll('[data-stat="downloads"]')
    .forEach(el => { el.textContent = s.downloads; });
  document.querySelectorAll('[data-stat="version"]')
    .forEach(el => { el.textContent = s.version; });
  if (s.msiUrl) {
    document.querySelectorAll('[data-action="download"]')
      .forEach(el => { el.href = s.msiUrl; });
  }
}

export async function fetchStats() {
  try {
    const cached = JSON.parse(localStorage.getItem(CACHE));
    if (cached && Date.now() - cached.ts < TTL) {
      applyStats(cached.data); return;
    }
  } catch (_) {}

  applyStats(FALLBACK);

  try {
    const H = { Accept: 'application/vnd.github+json' };
    const [rR, lR, aR] = await Promise.all([
      fetch(API, { headers: H }),
      fetch(`${API}/releases/latest`, { headers: H }),
      fetch(`${API}/releases?per_page=100`, { headers: H }),
    ]);
    if (!rR.ok || !lR.ok || !aR.ok) throw new Error('API error');
    const [repo, latest, all] = await Promise.all([rR.json(), lR.json(), aR.json()]);

    const stars     = fmt(repo.stargazers_count);
    const version   = latest.tag_name ?? FALLBACK.version;
    const totalDL   = all.reduce((t, r) =>
      t + (r.assets ?? []).reduce((s, a) => s + (a.download_count ?? 0), 0), 0);
    const downloads = fmt(totalDL);
    const msiAsset  = (latest.assets ?? []).find(a => a.name.endsWith('.msi'));
    const msiUrl    = msiAsset?.browser_download_url ?? FALLBACK.msiUrl;

    const stats = { stars, downloads, version, msiUrl };
    applyStats(stats);
    localStorage.setItem(CACHE, JSON.stringify({ ts: Date.now(), data: stats }));
  } catch (err) {
    console.warn('[PauseCat] Stats unavailable:', err.message);
  }
}
