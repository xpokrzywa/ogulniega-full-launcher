/* Ogulniega Mobile — logika aplikacji (bez frameworków, bez buildu) */
'use strict';

const APP_VERSION = '1.1.0';
const MIRROR = 'https://cdn.jsdelivr.net/gh/xMelonekMaX/ogulniega-distribution@main/';
const MODRINTH = 'https://api.modrinth.com/v2';

const state = {
	launcher: null,
	latest: null,
	versions: [],
	filter: '',
	clientCache: new Map(),
	modrinthToken: 0
};

const $ = (sel, root = document) => root.querySelector(sel);
const $$ = (sel, root = document) => Array.from(root.querySelectorAll(sel));

/* Tauri v2 odrzuca błędem komendy — dla wartości nie-stringowych może to być
   obiekt {kind, message}, nie Error. Czytaj każde z trzech kształtów. */
function errMsg(error) {
	if (error == null) return 'nieznany błąd';
	if (typeof error === 'string') return error;
	if (typeof error.message === 'string' && error.message) return error.message;
	try {
		return JSON.stringify(error);
	} catch {
		return String(error);
	}
}

function esc(value) {
	return String(value ?? '').replace(/[&<>"']/g, (ch) => ({
		'&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
	}[ch]));
}

function toast(message) {
	const el = $('#toast');
	el.textContent = message;
	el.hidden = false;
	clearTimeout(toast._t);
	toast._t = setTimeout(() => { el.hidden = true; }, 2600);
}

/* W APK z Tauri dane pobiera back-end w Rust (bez ograniczeń CORS).
   W przeglądarce zostaje zwykły fetch. */
const TAURI = globalThis.__TAURI_INTERNALS__ || globalThis.__TAURI__;

async function getJSON(url) {
	if (TAURI) {
		const invoke = TAURI.invoke || (TAURI.tauri && TAURI.tauri.invoke);
		if (invoke) {
			const body = await invoke('fetch_json', { url });
			return JSON.parse(body);
		}
	}
	const res = await fetch(url, { cache: 'no-cache' });
	if (!res.ok) throw new Error('HTTP ' + res.status);
	return res.json();
}

/* Pokazuje w zakładce Pomoc, czy działa backend Rust, czy tryb przeglądarki. */
async function describeBackend() {
	const el = $('#backend-line');
	if (!el) return;
	if (!TAURI) {
		el.textContent = 'Tryb: PWA (przeglądarka) — dane pobiera JavaScript.';
		return;
	}
	try {
		const info = await TAURI.invoke('backend_info');
		el.textContent = `Tryb: ${info.backend} • wersja ${info.appVersion} • ${info.platform}/${info.arch}`;
	} catch (err) {
		el.textContent = 'Tryb: Tauri (back-end niedostępny)';
	}
}

/* ---------- zakładki ---------- */

function showTab(name) {
	$$('.tab').forEach((t) => t.classList.toggle('active', t.id === 'tab-' + name));
	$$('.tabbtn').forEach((b) => b.classList.toggle('active', b.dataset.tab === name));
	window.scrollTo({ top: 0, behavior: 'smooth' });
}

/* ---------- dane ---------- */

async function loadData() {
	const [launcher, latest] = await Promise.allSettled([
		getJSON(MIRROR + 'launcher.json'),
		getJSON(MIRROR + 'launcher_version.json')
	]);

	if (launcher.status === 'fulfilled') {
		state.launcher = launcher.value;
		state.versions = Array.isArray(launcher.value.versions) ? launcher.value.versions : [];
	} else {
		$('#selected-version').textContent = 'błąd';
		$('#selected-details').innerHTML = '<span class="error">Nie udało się pobrać launcher.json — sprawdź internet.</span>';
	}

	if (latest.status === 'fulfilled') {
		state.latest = latest.value;
	}

	renderStart();
	renderVersions();
	fillModVersions();
	$('#last-updated').textContent = 'odświeżono: ' + new Date().toLocaleString('pl-PL');
}

function renderStart() {
	const launcher = state.launcher;
	if (launcher) {
		$('#selected-version').textContent = launcher.selected_version || '—';
		const sel = state.versions.find((v) => v.name === launcher.selected_version);
		$('#selected-details').textContent = sel
			? `Minecraft ${sel.minecraft_version} • ${sel.loader_name || 'vanilla'}`
			: 'wersja z launcher.json';
		$('#versions-count').textContent = String(state.versions.length);
		const mcs = [...new Set(state.versions.map((v) => v.minecraft_version).filter(Boolean))];
		$('#versions-range').textContent = mcs.length ? `Minecraft ${mcs[mcs.length - 1]} – ${mcs[0]}` : '—';

		const notices = Array.isArray(launcher.notices) ? launcher.notices : [];
		$('#notices').innerHTML = notices.length
			? notices.map((n) => `<li>${esc(typeof n === 'string' ? n : (n.message || JSON.stringify(n)))}</li>`).join('')
			: '<li class="muted">brak ogłoszeń</li>';
	}

	if (state.latest) {
		$('#launcher-latest').textContent = state.latest.version || '—';
	}
}

function versionMatches(version, phrase) {
	if (!phrase) return true;
	const hay = [version.name, version.minecraft_version, version.loader_name, version.java_name]
		.filter(Boolean).join(' ').toLowerCase();
	return phrase.toLowerCase().split(/\s+/).every((part) => hay.includes(part));
}

function renderVersions() {
	const list = $('#versions-list');
	if (!state.versions.length) {
		list.innerHTML = '<p class="muted">Brak danych — sprawdź połączenie i uruchom aplikację ponownie.</p>';
		return;
	}
	const filtered = state.versions.filter((v) => versionMatches(v, state.filter));
	if (!filtered.length) {
		list.innerHTML = '<p class="muted">Nic nie pasuje do filtra.</p>';
		return;
	}
	const selected = state.launcher && state.launcher.selected_version;
	list.innerHTML = filtered.map((v, index) => `
		<button class="row" data-version="${esc(v.name)}" data-index="${index}">
			<div class="row-main">
				<div class="row-title">${esc(v.name)} ${v.name === selected ? '<span class="chip">wybrana</span>' : ''}</div>
				<div class="row-sub">MC ${esc(v.minecraft_version || '?')} • ${esc(v.loader_name || 'vanilla')} • ${esc(v.java_name || 'java ?')}</div>
			</div>
			<span aria-hidden="true">›</span>
		</button>`).join('');

	$$('.row', list).forEach((btn) => btn.addEventListener('click', () => openVersion(btn.dataset.version)));
}

async function openVersion(name) {
	const version = state.versions.find((v) => v.name === name);
	if (!version) return;

	openSheet(`
		<h2>${esc(version.name)}</h2>
		<p class="muted">Minecraft ${esc(version.minecraft_version || '?')} • ${esc(version.loader_name || 'vanilla')}</p>
		<div class="card"><h3>Java</h3><p class="muted small">${esc(version.java_name || 'nie podano')}</p></div>
		<div class="card"><h3>Argumenty JVM</h3><p class="muted small">${esc((version.jvm_args || []).join(' ') || 'brak')}</p></div>
		<div id="sheet-mods"><div class="skeleton"></div></div>
	`);

	let data;
	try {
		if (!state.clientCache.has(name)) {
			state.clientCache.set(name, await getJSON(MIRROR + 'client_versions/' + encodeURIComponent(name) + '.json'));
		}
		data = state.clientCache.get(name);
	} catch (error) {
		$('#sheet-mods').innerHTML = `<p class="error">Nie udało się pobrać listy plików tej wersji (${esc(error.message)}).</p>`;
		return;
	}

	const mods = Array.isArray(data.mods) ? data.mods : [];
	const links = [
		data.java_url ? `<a class="btn small" href="${esc(data.java_url)}" target="_blank" rel="noopener">Java Windows</a>` : '',
		data.java_linux_url ? `<a class="btn small" href="${esc(data.java_linux_url)}" target="_blank" rel="noopener">Java Linux</a>` : ''
	].filter(Boolean).join(' ');

	$('#sheet-mods').innerHTML = `
		<div class="card">
			<h3>Pliki wersji</h3>
			<p>${mods.length} modów${data.loader_url ? ' + loader' : ''}</p>
			<div class="mod-actions">${links}</div>
			<div class="actions" style="margin-top:12px">
				<button class="btn primary" id="btn-graj">Graj</button>
			</div>
			<p id="graj-status" class="muted small"></p>
		</div>
		<div class="card">
			<h3>Lista modów</h3>
			<ul class="mods-mini">
				${mods.slice(0, 80).map((mod) => `
					<li><span>${esc(mod.name || mod.id)}</span>
					<a href="${esc(mod.url || mod.fallback_url || '#')}" target="_blank" rel="noopener">pobierz</a></li>`).join('')}
			</ul>
			${mods.length > 80 ? `<p class="muted small">…i ${mods.length - 80} więcej</p>` : ''}
		</div>
	`;

	/* Pobranie plików wersji: równolegle (4 równolegle), weryfikacja SHA-512
	   po stronie Rust, ścieżki względne (backend skleja z katalogiem danych).
	   Argumenty komend w camelCase — domyślna konwencja Tauri v2. */
	$('#btn-graj').addEventListener('click', async () => {
		const invoke = TAURI && (TAURI.invoke || (TAURI.tauri && TAURI.tauri.invoke));
		if (!invoke) {
			toast('Pobieranie plików działa tylko w APK.');
			return;
		}
		const status = $('#graj-status');
		const set = (text) => { if (status) status.textContent = text; };
		try {
			set('Pobieranie manifestu wersji…');
			const data = await getJSON(MIRROR + 'client_versions/' + encodeURIComponent(name) + '.json');
			const mods = Array.isArray(data.mods) ? data.mods : [];
			const dir = 'ogulniega/' + name;
			const jobs = [];
			if (data.java_linux_url) {
				jobs.push({ url: data.java_linux_url, dest: dir + '/java-linux.tar.gz', sha: null });
			}
			for (const mod of mods) {
				const url = mod.url || mod.fallback_url;
				if (!url) continue;
				jobs.push({
					url,
					fallback: mod.fallback_url && mod.fallback_url !== url ? mod.fallback_url : null,
					dest: dir + '/mods/' + (mod.name || mod.id),
					sha: mod.sha512 || null
				});
			}
			if (!jobs.length) {
				set('Manifest nie zawiera plików do pobrania.');
				return;
			}

			let done = 0;
			let failed = 0;
			let firstError = '';
			let cursor = 0;

			const fetchOne = async (job) => {
				try {
					await invoke('download_file', {
						url: job.url,
						destPath: job.dest,
						expectedSha512: job.sha
					});
				} catch (error) {
					if (job.fallback) {
						try {
							await invoke('download_file', {
								url: job.fallback,
								destPath: job.dest,
								expectedSha512: job.sha
							});
						} catch (error2) {
							failed++;
							if (!firstError) firstError = errMsg(error2);
						}
					} else {
						failed++;
						if (!firstError) firstError = errMsg(error);
					}
				}
				done++;
				set(`Pobieranie ${done}/${jobs.length}${failed ? `, błędy: ${failed}` : ''}…`);
			};

			const worker = async () => {
				while (cursor < jobs.length) {
					await fetchOne(jobs[cursor++]);
				}
			};
			const lanes = Math.min(4, jobs.length);
			await Promise.all(Array.from({ length: lanes }, () => worker()));

			if (failed) {
				set(`Zakończone: ${done - failed}/${jobs.length}. Pierwszy błąd: ${firstError}`);
				toast(`Pobieranie: ${failed} błędów`);
			} else {
				set(`Gotowe: ${jobs.length} plików, wszystkie zweryfikowane SHA-512.`);
				toast('Pobieranie zakończone — pliki gotowe.');
			}
		} catch (error) {
			set('Błąd: ' + errMsg(error));
		}
	});
}

/* ---------- Modrinth ---------- */

function fillModVersions() {
	const select = $('#mod-version');
	const versions = [...new Set(state.versions.map((v) => v.minecraft_version).filter(Boolean))]
		.sort((a, b) => b.localeCompare(a, undefined, { numeric: true }));
	select.innerHTML = ['<option value="">dowolna wersja</option>']
		.concat(versions.map((v) => `<option value="${esc(v)}">${esc(v)}</option>`)).join('');
}

async function searchMods(query) {
	const token = ++state.modrinthToken;
	const list = $('#mods-list');
	const version = $('#mod-version').value;

	if (!query.trim()) {
		list.innerHTML = '<p class="muted">Wpisz nazwę moda, np. „sodium”, „lithium”, „journeymap”.</p>';
		return;
	}

	const facets = [['project_type:mod'], ['categories:fabric']];
	if (version) facets.push([`versions:${version}`]);
	const params = new URLSearchParams({
		query: query.trim(),
		facets: JSON.stringify(facets),
		limit: '12',
		index: 'relevance'
	});

	list.innerHTML = '<div class="skeleton"></div><div class="skeleton"></div>';

	let data;
	try {
		data = await getJSON(`${MODRINTH}/search?${params.toString()}`);
	} catch (error) {
		if (token === state.modrinthToken) {
			list.innerHTML = `<p class="error">Modrinth nie odpowiedział (${esc(error.message)}).</p>`;
		}
		return;
	}
	if (token !== state.modrinthToken) return;

	if (!data.hits || !data.hits.length) {
		list.innerHTML = '<p class="muted">Brak wyników — spróbuj innej nazwy lub innej wersji Minecrafta.</p>';
		return;
	}

	list.innerHTML = data.hits.map((hit) => `
		<article class="mod-card">
			<img src="${esc(hit.icon_url || './icons/icon-192.png')}" alt="" loading="lazy" />
			<div class="row-main">
				<div class="row-title">${esc(hit.title)}</div>
				<div class="row-sub">${esc(hit.description || '')}</div>
				<div class="row-sub">↓ ${esc(Number(hit.downloads || 0).toLocaleString('pl-PL'))} • ${esc(hit.client_side || '')}</div>
				<div class="mod-actions">
					<a class="btn small" href="https://modrinth.com/mod/${esc(hit.slug)}" target="_blank" rel="noopener">Strona moda</a>
					<button class="btn small" data-file="${esc(hit.project_id)}" data-title="${esc(hit.title)}">Plik dla wersji</button>
				</div>
			</div>
		</article>`).join('');

	$$('[data-file]', list).forEach((btn) => btn.addEventListener('click', () => {
		openModFile(btn.dataset.file, btn.dataset.title);
	}));
}

async function openModFile(projectId, title) {
	const version = $('#mod-version').value;
	const params = new URLSearchParams({ loaders: '["fabric"]' });
	if (version) params.set('game_versions', `["${version}"]`);

	toast('Szukam pliku…');
	let files;
	try {
		files = await getJSON(`${MODRINTH}/project/${encodeURIComponent(projectId)}/version?${params.toString()}`);
	} catch (error) {
		toast('Błąd Modrinth: ' + error.message);
		return;
	}

	if (!Array.isArray(files) || !files.length) {
		openSheet(`<h2>${esc(title)}</h2><p class="muted">Brak plików Fabric${version ? ' dla Minecrafta ' + esc(version) : ''}.</p>`);
		return;
	}

	const latest = files[0];
	const primary = (latest.files || []).find((file) => file.primary) || (latest.files || [])[0] || {};
	openSheet(`
		<h2>${esc(title)}</h2>
		<p class="muted">Wersja moda: ${esc(latest.version_number || '?')} • Minecraft ${esc((latest.game_versions || []).slice(-3).join(', '))}</p>
		${primary.url ? `<a class="btn primary" href="${esc(primary.url)}" target="_blank" rel="noopener">Pobierz ${esc(primary.filename || 'plik')}</a>` : '<p class="muted">Brak pliku do pobrania.</p>'}
		<p class="muted small" style="margin-top:14px">Plik .jar wrzuć do folderu mods swojego launchera na Androidzie (np. PojavLauncher) —
			wtedy zadziała. Ta aplikacja tylko podaje link.</p>
	`);
}

/* ---------- arkusz dolny ---------- */

function openSheet(html) {
	$('#sheet-body').innerHTML = html;
	$('#sheet').hidden = false;
}

function closeSheet() {
	$('#sheet').hidden = true;
	$('#sheet-body').innerHTML = '';
}

/* ---------- PWA ---------- */

let deferredPrompt = null;

function setupInstall() {
	window.addEventListener('beforeinstallprompt', (event) => {
		event.preventDefault();
		deferredPrompt = event;
		$('#install-btn').hidden = false;
	});

	$('#install-btn').addEventListener('click', async () => {
		if (!deferredPrompt) {
			toast('Użyj menu Chrome ⋮ → „Dodaj do ekranu głównego”.');
			return;
		}
		deferredPrompt.prompt();
		await deferredPrompt.userChoice;
		deferredPrompt = null;
		$('#install-btn').hidden = true;
	});
}

function setupServiceWorker() {
	if (!('serviceWorker' in navigator)) return;
	if (location.protocol === 'file:') return;
	navigator.serviceWorker.register('./sw.js').catch(() => { /* offline i tak zadziała po drugim wejściu */ });
}

/* ---------- start ---------- */

function repoLinks() {
	if (!location.hostname.endsWith('github.io')) return;
	const owner = location.hostname.split('.')[0];
	const repo = location.pathname.split('/').filter(Boolean)[0];
	if (!owner || !repo) return;
	const base = `https://github.com/${owner}/${repo}`;
	$('#repo-links').innerHTML =
		`Repozytorium: <a href="${esc(base)}" target="_blank" rel="noopener">${esc(owner + '/' + repo)}</a> • ` +
		`<a href="${esc(base)}/releases" target="_blank" rel="noopener">wydania (APK)</a> • ` +
		`<a href="${esc(base)}/actions" target="_blank" rel="noopener">Actions</a>`;
}

function init() {
	$('#version-line').textContent = 'wersja aplikacji ' + APP_VERSION;
	describeBackend();
	repoLinks();

	$$('.tabbtn').forEach((btn) => btn.addEventListener('click', () => showTab(btn.dataset.tab)));
	$('#sheet').addEventListener('click', (event) => {
		if (event.target.id === 'sheet' || event.target.id === 'sheet-close') closeSheet();
	});

	$('#version-filter').addEventListener('input', (event) => {
		state.filter = event.target.value;
		renderVersions();
	});

	let modTimer;
	$('#mod-query').addEventListener('input', (event) => {
		clearTimeout(modTimer);
		const value = event.target.value;
		modTimer = setTimeout(() => searchMods(value), 350);
	});
	$('#mod-version').addEventListener('change', () => {
		if ($('#mod-query').value.trim()) searchMods($('#mod-query').value);
	});

	setupInstall();
	setupServiceWorker();
	loadData();

	window.addEventListener('online', loadData);
}

document.addEventListener('DOMContentLoaded', init);
