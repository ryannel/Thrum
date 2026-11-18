// Adapter for the existing headless Chrome launcher and its local HTTP origin.
import initialize, { capture as captureImage, restore as restoreImage } from "./match_proof.js";

let images;
let proof;

export default async function initializeProof() {
    const manifest = await fetch("./images.json").then(response => response.json());
    images = new Map(await Promise.all(manifest.images.map(async name => {
        const response = await fetch(name);
        if (!response.ok) throw new Error(`Cannot fetch Mac image: ${name}`);
        return [name, new Uint8Array(await response.arrayBuffer())];
    })));
    if (manifest.requests.length) {
        proof = await runBatch(manifest.requests);
    } else {
        await initialize();
    }
}

function runWorker(request) {
    const names = request.method === "capture" ? request.args[3] : [request.args[2]];
    const references = new Map(names.map(name => [name, images.get(name)]));
    const worker = new Worker(new URL("./image_worker.js", import.meta.url), { type: "module" });
    return new Promise((resolve, reject) => {
        worker.onmessage = ({ data }) => {
            if (data.error) reject(new Error(data.error));
            else resolve(data.result);
        };
        worker.onerror = event => reject(new Error(event.message));
        worker.onmessageerror = () => reject(new Error("Cannot decode image worker result"));
        worker.postMessage({ request, references });
    }).finally(() => worker.terminate());
}

async function runBatch(requests) {
    const results = new Array(requests.length);
    let next = 0;
    async function run() {
        while (next < requests.length) {
            const index = next++;
            results[index] = await runWorker(requests[index]);
        }
    }
    await Promise.all(Array.from({ length: Math.min(4, requests.length) }, run));
    return results;
}

export function proveImages() {
    return JSON.stringify(proof);
}

export async function runImageRequest(request, references) {
    await initialize();
    images = references;
    const run = request.method === "capture" ? capture : restore;
    return JSON.parse(run(...request.args));
}

export function capture(count, mixed, ticks, names) {
    const packet = JSON.parse(captureImage(count, mixed, Uint32Array.from(ticks)));
    if (packet.rows.length !== names.length) throw new Error("Missing captured images");
    for (const [index, row] of packet.rows.entries()) {
        const reference = images.get(names[index]);
        if (row.image.length !== reference.length * 2) {
            throw new Error(`Image size differs at tick ${row.tick}`);
        }
        for (const [offset, byte] of reference.entries()) {
            const actual = Number.parseInt(row.image.slice(offset * 2, offset * 2 + 2), 16);
            if (actual !== byte) {
                throw new Error(`Image differs at tick ${row.tick}, byte ${offset}: Mac ${byte}, Chrome ${actual}`);
            }
        }
        delete row.image;
        row.equal_to_mac = true;
    }
    return JSON.stringify(packet);
}

export function restore(count, mixed, name, last) {
    return restoreImage(count, mixed, images.get(name), last);
}
