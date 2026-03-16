const BASE = 'https://localhost:8080';

const tests = [
	{ method: 'GET', path: '/hello',      expect: 200 },
	{ method: 'GET', path: '/about',      expect: 200 },
	{ method: 'GET', path: '/text',       expect: 200 },
	{ method: 'GET', path: '/old-about',  expect: [301, 302, 307, 308], note: 'redirect' },
	{ method: 'GET', path: '/no-content', expect: 204 },
	{ method: 'GET', path: '/api/test',   expect: 200, note: 'CORS middleware' },
	{ method: 'GET', path: '/not-a-page', expect: 404, note: 'custom error page' },
];

// ── Build cards ──────────────────────────────────────────────────────────────

function buildCards() {
	const grid = document.getElementById('cards');

	tests.forEach((t, i) => {
		const card = document.createElement('div');
		card.className = 'card';
		card.id = `card-${i}`;
		card.innerHTML = `
            <div class="card-top">
                <span class="status-dot" id="dot-${i}"></span>
                <span class="method-badge">${t.method}</span>
                <span class="card-label">${t.note ?? ''}</span>
            </div>
            <div class="endpoint">${t.path}</div>
            <button class="send-btn" onclick="runTest(${i})">Send Request</button>
            <div class="result" id="result-${i}"></div>
        `;
		grid.appendChild(card);
	});
}

// ── Run a single test ────────────────────────────────────────────────────────

async function runTest(i) {
	const t = tests[i];
	const dot    = document.getElementById(`dot-${i}`);
	const result = document.getElementById(`result-${i}`);

	dot.className    = 'status-dot pending';
	result.style.display = 'block';
	result.className = 'result info';
	result.textContent = 'Sending...';

	const start = Date.now();

	try {
		const res = await fetch(BASE + t.path, {
			method: t.method,
			redirect: 'manual',
		});

		const elapsed  = Date.now() - start;
		const expected = Array.isArray(t.expect) ? t.expect : [t.expect];
		const ok       = expected.includes(res.status);

		let body = '';
		try { body = await res.text(); } catch {}
		if (body.length > 200) body = body.slice(0, 200) + '…';

		dot.className    = 'status-dot ' + (ok ? 'ok' : 'fail');
		result.className = 'result '     + (ok ? 'success' : 'error');
		result.textContent = `HTTP ${res.status} · ${elapsed}ms\n\n${body}`;

		addLog(t.path, res.status, elapsed, ok);

	} catch (err) {
		const elapsed = Date.now() - start;
		dot.className    = 'status-dot fail';
		result.className = 'result error';
		result.textContent = `Network error\n${err.message}`;
		addLog(t.path, 'ERR', elapsed, false);
	}
}

// ── Run all tests sequentially ───────────────────────────────────────────────

async function runAll() {
	for (let i = 0; i < tests.length; i++) {
		await runTest(i);
	}
}

// ── Log ──────────────────────────────────────────────────────────────────────

function addLog(path, status, ms, ok) {
	document.getElementById('log-empty').style.display = 'none';

	const log   = document.getElementById('log');
	const entry = document.createElement('div');
	const time  = new Date().toLocaleTimeString();

	entry.className = 'log-entry';
	entry.innerHTML = `
        <span class="log-time">${time}</span>
        <span class="log-status ${ok ? 'ok' : 'fail'}">${status}</span>
        <span class="log-path">${path}</span>
        <span class="log-ms">${ms}ms</span>
    `;
	log.prepend(entry);
}

function clearLog() {
	document.getElementById('log').innerHTML = '';
	document.getElementById('log-empty').style.display = 'block';
}

// ── Init ─────────────────────────────────────────────────────────────────────

buildCards();