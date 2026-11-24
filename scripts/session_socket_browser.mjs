#!/usr/bin/env node
// Start the real Chrome target. CDP only controls reload and page lifecycle.
import fs from 'node:fs/promises';
import path from 'node:path';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

const [directory, artifacts, log, control, pageControl, ticket, bodies, axes] = process.argv.slice(2);
const chromePath = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
const page = '<!doctype html><title>Thrum socket match</title><script type="module" src="/socket_player.js"></script>';
const staticFiles = new Set(['match_proof.js', 'match_proof_bg.wasm', 'socket_player.js']);
const readControl = async file => (await fs.readFile(file, 'utf8').catch(() => '')).trim();
const writeRows = rows => fs.appendFile(log, rows.map(row => JSON.stringify(row) + '\n').join(''));
const server = createServer(async (request, response) => {
    try {
        if (request.url === '/') {
            response.setHeader('Content-Type', 'text/html');
            response.end(page);
        } else if (request.url === '/config') {
            response.setHeader('Content-Type', 'application/json');
            response.end(JSON.stringify({ ticket, bodies: Number(bodies), axes: axes === 'true' }));
        } else if (request.url === '/rows' && request.method === 'POST') {
            let data = '';
            for await (const chunk of request) {
                data += chunk;
                if (data.length > 1024 * 1024) throw new Error('proof record exceeded its upload limit');
            }
            await writeRows(JSON.parse(data));
            response.end(await readControl(control));
        } else if (staticFiles.has(request.url.slice(1))) {
            response.setHeader('Content-Type', request.url.endsWith('.wasm') ? 'application/wasm' : 'text/javascript');
            response.end(await fs.readFile(path.join(artifacts, request.url.slice(1))));
        } else {
            response.writeHead(404).end();
        }
    } catch (error) {
        response.writeHead(500).end(String(error));
    }
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const flags = ['--headless=new', '--remote-debugging-port=0', `--user-data-dir=${directory}`,
    '--no-first-run', 'about:blank'];
const chrome = spawn(chromePath, flags, { detached: true, stdio: ['ignore', 'ignore', 'inherit'] });
let pageSession;
let browserSession;
const frozen = new Set();
let interrupted = false;
for (const signal of ['SIGTERM', 'SIGINT', 'SIGHUP']) {
    process.once(signal, () => { interrupted = true; });
}

function resumeRenderers() {
    for (const pid of frozen) {
        try { process.kill(pid, 'SIGCONT'); }
        catch (error) { if (error.code !== 'ESRCH') throw error; }
    }
    frozen.clear();
}

async function connect(url) {
    const socket = new WebSocket(url);
    await new Promise((resolve, reject) => {
        socket.addEventListener('open', resolve, { once: true });
        socket.addEventListener('error', reject, { once: true });
    });
    const pending = new Map();
    let sequence = 0;
    socket.addEventListener('message', message => {
        const reply = JSON.parse(message.data);
        if (reply.method === 'Runtime.exceptionThrown') {
            const problem = reply.params.exceptionDetails;
            writeRows([{ event: 'error', reason: problem.exception?.description ?? problem.text }])
                .catch(error => { process.stderr.write(String(error)); });
        }
        const entry = pending.get(reply.id);
        if (!entry) return;
        clearTimeout(entry.timer);
        pending.delete(reply.id);
        if (reply.error) entry.reject(new Error(JSON.stringify(reply.error)));
        else entry.resolve(reply.result);
    });
    const call = (method, params = {}) => new Promise((resolve, reject) => {
        const id = ++sequence;
        const timer = setTimeout(() => {
            pending.delete(id);
            reject(new Error(`DevTools timeout: ${method}`));
        }, 10_000);
        pending.set(id, { resolve, reject, timer });
        socket.send(JSON.stringify({ id, method, params }));
    });
    return { call, close: () => socket.close() };
}

try {
    let port;
    for (let attempt = 0; attempt < 150 && !interrupted; attempt++) {
        const file = await fs.readFile(path.join(directory, 'DevToolsActivePort'), 'utf8').catch(() => '');
        port = Number(file.split('\n')[0]);
        if (port) break;
        if (chrome.exitCode !== null) throw new Error('Chrome exited before opening DevTools');
        await delay(100);
    }
    if (!port) throw new Error('Chrome DevTools did not start');
    const tabs = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
    pageSession = await connect(tabs.find(tab => tab.type === 'page').webSocketDebuggerUrl);
    const version = await pageSession.call('Browser.getVersion');
    await pageSession.call('Runtime.enable');
    await writeRows([{ event: 'client', pid: chrome.pid, driver_pid: process.pid,
        bodies: Number(bodies), version, flags, wall_us: Date.now() * 1000 }]);
    await pageSession.call('Page.navigate', { url: `http://127.0.0.1:${server.address().port}/` });
    let prior = '';
    while (!interrupted && chrome.exitCode === null) {
        const command = await readControl(pageControl);
        if (command !== prior) {
            if (command === 'exit') break;
            if (command === 'reload') await pageSession.call('Page.reload');
            if (command === 'frozen') {
                // Chrome's page-lifecycle transition closes WebTransport.
                // Suspend the isolated renderer as TRANSPORT's proof does,
                // leaving Chrome's network service alive for the session test.
                browserSession ??= await connect((await (await fetch(
                    `http://127.0.0.1:${port}/json/version`)).json()).webSocketDebuggerUrl);
                const info = await browserSession.call('SystemInfo.getProcessInfo');
                for (const renderer of info.processInfo.filter(entry => entry.type === 'renderer')) {
                    process.kill(renderer.id, 'SIGSTOP');
                    frozen.add(renderer.id);
                }
                if (!frozen.size) throw new Error('Chrome reported no renderer to freeze');
            }
            const renderer_pids = Array.from(frozen);
            if (command === 'active') resumeRenderers();
            if (command) await writeRows([{ event: 'page', command, renderer_pids, wall_us: Date.now() * 1000 }]);
            prior = command;
        }
        await delay(20);
    }
    if (!interrupted && chrome.exitCode !== null) throw new Error('Chrome exited during the socket match');
} finally {
    resumeRenderers();
    pageSession?.close();
    browserSession?.close();
    server.closeAllConnections();
    server.close();
    try { process.kill(-chrome.pid, 'SIGTERM'); }
    catch (error) { if (error.code !== 'ESRCH') throw error; }
    for (let count = 0; count < 10 && chrome.exitCode === null && chrome.signalCode === null; count++) await delay(100);
    try { process.kill(-chrome.pid, 'SIGKILL'); }
    catch (error) { if (error.code !== 'ESRCH') throw error; }
    chrome.unref();
}
