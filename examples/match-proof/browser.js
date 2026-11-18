// Adapter for the existing headless Chrome launcher and its local HTTP origin.
import initialize, { capture as captureImage, restore as restoreImage } from "./match_proof.js";

let images;

export default async function initializeProof() {
    await initialize();
    const manifest = await fetch("./images.json").then(response => response.json());
    images = new Map(await Promise.all(manifest.map(async name => {
        const response = await fetch(name);
        if (!response.ok) throw new Error(`Cannot fetch Mac image: ${name}`);
        return [name, new Uint8Array(await response.arrayBuffer())];
    })));
}

export function capture(count, mixed, ticks) {
    return captureImage(count, mixed, Uint32Array.from(ticks));
}

export function restore(count, mixed, name, last) {
    return restoreImage(count, mixed, images.get(name), last);
}
