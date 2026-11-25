import init, { WindowedMatch } from './match_proof.js';

const config = await (await fetch('/config')).json();
const canvas = document.querySelector('canvas');
canvas.width = 640;
canvas.height = 360;
const rows = [];
let player, running = true, command = '', capture = 'idle', imageCount = 0;
let latestVisibility = 0;
const pending = [];
const available = Array.from({ length: 16 }, () => new Uint8Array(canvas.width * canvas.height * 4));
let uploading = false;

async function upload() {
    if (uploading) return;
    uploading = true;
    try {
        while (pending.length) {
            const { metadata, pixels } = pending.shift();
            const result = await fetch('/image', { method: 'POST', body: pixels });
            if (!result.ok) throw new Error(await result.text());
            available.push(pixels);
            rows.push(metadata);
            imageCount += 1;
        }
    } catch (error) {
        fail(error);
    } finally {
        uploading = false;
    }
}

function collect() {
    while (available.length) {
        const pixels = available.pop();
        const raw = player.capture(pixels);
        if (raw === undefined) {
            available.push(pixels);
            break;
        }
        pending.push({ metadata: JSON.parse(raw), pixels });
    }
    void upload();
}

function fail(error) {
    running = false;
    rows.push({ event: 'error', reason: String(error.stack ?? error) });
}

async function flush() {
    while (running || rows.length) {
        if (rows.length) {
            const result = await fetch('/rows', { method: 'POST', body: JSON.stringify(rows.splice(0)) });
            if (!result.ok) throw new Error(await result.text());
            command = await result.text();
        }
        await new Promise(resolve => setTimeout(resolve, 60));
    }
}

function visibility() {
    try {
        latestVisibility = performance.now();
        player.visible(!document.hidden, latestVisibility);
        rows.push({ event: 'visibility', hidden: document.hidden, wall_us: Date.now() * 1000 });
    } catch (error) {
        fail(error);
    }
}

function frame(timestamp) {
    if (!running) return;
    try {
        if (capture === 'idle' && command === 'record') {
            player.begin_capture();
            capture = 'recording';
        }
        if (capture === 'recording' && command === 'stop') {
            player.stop_capture();
            capture = 'draining';
        }
        player.frame(Math.max(timestamp, latestVisibility));
        rows.push(...JSON.parse(player.rows()));
        if (capture === 'recording' || capture === 'draining') collect();
        if (capture === 'draining' && !player.pending_capture() && !uploading && !pending.length) {
            capture = 'finished';
            rows.push({ event: 'capture_finished', mode: 'realtime', frames: imageCount, omitted: Number(player.omissions()) });
        }
        requestAnimationFrame(frame);
    } catch (error) {
        fail(error);
    }
}

void flush();
try {
    await init();
    player = await WindowedMatch.open(canvas, config.ticket);
    rows.push({ event: 'webgpu', adapter: player.adapter() });
    document.addEventListener('visibilitychange', visibility);
    visibility();
    requestAnimationFrame(frame);
} catch (error) {
    fail(error);
}
