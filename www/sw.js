/* Service worker: powłoka aplikacji z cache, dane z sieci z fallbackiem offline */
const CACHE = 'ogulniega-mobile-v2';
const SHELL = [
	'./',
	'./index.html',
	'./app.js',
	'./style.css',
	'./manifest.webmanifest',
	'./icons/icon-192.png',
	'./icons/icon-512.png',
	'./fonts/PressStart2P-Regular.ttf'
];

self.addEventListener('install', (event) => {
	event.waitUntil(
		caches.open(CACHE)
			.then((cache) => cache.addAll(SHELL))
			.then(() => self.skipWaiting())
	);
});

self.addEventListener('activate', (event) => {
	event.waitUntil(
		caches.keys()
			.then((keys) => Promise.all(keys.filter((key) => key !== CACHE).map((key) => caches.delete(key))))
			.then(() => self.clients.claim())
	);
});

self.addEventListener('fetch', (event) => {
	const request = event.request;
	if (request.method !== 'GET') return;

	const url = new URL(request.url);
	const isData = url.hostname.endsWith('jsdelivr.net') || url.hostname === 'api.modrinth.com';

	if (isData) {
		// dane: najpierw sieć, w razie błędu ostatnia kopia
		event.respondWith(
			fetch(request)
				.then((response) => {
					const copy = response.clone();
					caches.open(CACHE).then((cache) => cache.put(request, copy));
					return response;
				})
				.catch(() => caches.match(request))
		);
		return;
	}

	if (url.origin === location.origin) {
		if (url.pathname.endsWith('.ttf')) {
			// czcześciowo: fonty pobierane przez cache API, ale brak offline
			event.respondWith(fetch(request).then((response) => {
				const copy = response.clone();
				caches.open(CACHE).then((cache) => cache.put(request, copy));
				return response;
			}));
			return;
		}
		// powłoka: cache-first, dociąganie w tle
		event.respondWith(
			caches.match(request).then((cached) => cached || fetch(request).then((response) => {
				const copy = response.clone();
				caches.open(CACHE).then((cache) => cache.put(request, copy));
				return response;
			}))
		);
	}
});
