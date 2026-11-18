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
