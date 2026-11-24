// The page supplies a clock, controls persistence, and records Rust's observations.
import initialize, { SocketPlayer } from './match_proof.js';

await initialize();
const { ticket, bodies, axes } = await (await fetch('/config')).json();
while ((await (await fetch('/rows', { method: 'POST', body: '[]' })).text()) === 'hold') {
    await new Promise(resolve => setTimeout(resolve, 25));
}
let saved = sessionStorage.getItem('player token') ?? '';
let player = new SocketPlayer(ticket, bodies, saved, axes);
let nextRecord = 0;
let command = '';
let pending = [];
let uploading = false;
const nextTurn = new MessageChannel();
nextTurn.port1.onmessage = () => poll().catch(report);

async function poll() {
    const before = player.tick();
    pending.push(...JSON.parse(player.advance()));
    const token = player.token();
    if (token && token !== saved) {
        sessionStorage.setItem('player token', token);
        saved = token;
    }
    if (!uploading && performance.now() >= nextRecord) {
        nextRecord = performance.now() + 75;
        uploading = true;
        const batch = pending;
        pending = [];
        upload(batch).catch(report).finally(() => { uploading = false; });
    }
    if (player.tick() > before + 1n) nextTurn.port2.postMessage(null);
    else setTimeout(() => poll().catch(report), 1);
}

async function upload(batch) {
    const response = await fetch('/rows', { method: 'POST', body: JSON.stringify(batch) });
    if (!response.ok) throw new Error(await response.text());
    const next = await response.text();
    if (next !== command) {
        if (next === 'reconnect') {
            player.free();
            player = new SocketPlayer(ticket, bodies, saved, axes);
        }
        if (next === 'perturb') pending.push(JSON.parse(player.perturb()));
        command = next;
    }
}

function report(error) {
    return fetch('/rows', { method: 'POST', body: JSON.stringify([{ event: 'error', reason: String(error) }]) });
}

await poll().catch(report);
