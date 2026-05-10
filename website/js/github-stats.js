const REPO = '0xarchit/pauseCat';
const CACHE_KEY = 'pausecat_gh_stats';
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes

const FALLBACKS = {
  stars:     '—',
  downloads: '—',
  version:   '—',
  msiUrl:    'https://github.com/0xarchit/pauseCat/releases/latest',
};

function formatNumber(n) {
  if (typeof n !== 'number') return '—';
  if (n >= 1000) return (n / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
  return n.toLocaleString();
}

function applyStats(stats) {
  document.querySelectorAll('[data-stat="stars"]')
    .forEach(el => el.textContent = stats.stars);
  document.querySelectorAll('[data-stat="downloads"]')
    .forEach(el => el.textContent = stats.downloads);
  document.querySelectorAll('[data-stat="version"]')
    .forEach(el => el.textContent = stats.version);

  // Update all download links to point to the real MSI asset
  if (stats.msiUrl) {
    document.querySelectorAll('[data-action="download"]')
      .forEach(el => el.href = stats.msiUrl);
  }
}

async function fetchStats() {
  // ── 1. Check cache ─────────────────────────────────────────────
  try {
    const cached = JSON.parse(localStorage.getItem(CACHE_KEY) || 'null');
    if (cached && Date.now() - cached.ts < CACHE_TTL_MS) {
      applyStats(cached.data);
      return;
    }
  } catch (_) { /* corrupt cache — ignore, re-fetch */ }

  // ── 2. Apply fallbacks immediately so page never shows empty ───
  applyStats(FALLBACKS);

  // ── 3. Fetch in parallel ───────────────────────────────────────
  try {
    const headers = { 'Accept': 'application/vnd.github+json' };

    const [repoRes, latestRes, allReleasesRes] = await Promise.all([
      fetch(`https://api.github.com/repos/${REPO}`,                    { headers }),
      fetch(`https://api.github.com/repos/${REPO}/releases/latest`,    { headers }),
      fetch(`https://api.github.com/repos/${REPO}/releases?per_page=100`, { headers }),
    ]);

    if (!repoRes.ok || !latestRes.ok || !allReleasesRes.ok) {
      throw new Error('One or more API requests failed');
    }

    const [repo, latest, allReleases] = await Promise.all([
      repoRes.json(),
      latestRes.json(),
      allReleasesRes.json(),
    ]);

    // ── Stars ────────────────────────────────────────────────────
    const stars = formatNumber(repo.stargazers_count);

    // ── Version ──────────────────────────────────────────────────
    const version = latest.tag_name || FALLBACKS.version;

    // ── Total downloads: sum every asset across every release ────
    const totalDownloads = Array.isArray(allReleases)
      ? allReleases.reduce((total, release) =>
          total + (release.assets || []).reduce((sum, asset) =>
            sum + (asset.download_count || 0), 0), 0)
      : 0;
    const downloads = formatNumber(totalDownloads);

    // ── MSI download URL from latest release ────────────────────
    const msiAsset = (latest.assets || []).find(a => a.name.endsWith('.msi'));
    const msiUrl = msiAsset?.browser_download_url || FALLBACKS.msiUrl;

    const stats = { stars, downloads, version, msiUrl };

    // ── 4. Update DOM ─────────────────────────────────────────────
    applyStats(stats);

    // ── 5. Cache result ──────────────────────────────────────────
    localStorage.setItem(CACHE_KEY, JSON.stringify({ ts: Date.now(), data: stats }));

  } catch (err) {
    // Fallbacks already applied in step 2 — nothing more to do
    console.warn('[PauseCat] GitHub stats unavailable:', err.message);
  }
}

export { fetchStats };