import { readFile } from 'node:fs/promises';
import { performance } from 'node:perf_hooks';

const [, , path, mode, pullSize] = process.argv;
const module = await WebAssembly.compile(await readFile(path));
const imports = WebAssembly.Module.imports(module);
if (imports.length !== 0) {
    throw new Error(`the audio module imports ${JSON.stringify(imports)}`);
}

const exports = (await WebAssembly.instantiate(module)).exports;
if (mode === 'costs') {
    exports.audio_cost_prepare();
    const start = performance.now();
    const witness = exports.audio_cost_render() >>> 0;
    console.log(JSON.stringify({ render_seconds: (performance.now() - start) / 1000, witness }));
} else if (mode === 'reference' || mode === 'reference-bits') {
    const frames = Number(pullSize);
    if (!Number.isInteger(frames) || frames < 1 || frames > 4096) {
        throw new Error('expected a pull size in 1 to 4096');
    }
    exports.audio_reference_prepare();
    let hash = 0xcbf29ce484222325n;
    let count;
    while ((count = exports.audio_reference_pull(frames)) !== 0) {
        const offset = exports.audio_output_samples();
        const samples = new Uint8Array(exports.memory.buffer, offset, count * 8);
        if (mode === 'reference-bits') {
            const words = new DataView(samples.buffer, samples.byteOffset, samples.byteLength);
            for (let i = 0; i < words.byteLength; i += 4) {
                console.log(words.getUint32(i, true).toString(16).padStart(8, '0'));
            }
        } else {
            for (const byte of samples) {
                hash = BigInt.asUintN(64, (hash ^ BigInt(byte)) * 0x100000001b3n);
            }
        }
    }
    if (mode === 'reference') {
        console.log(hash.toString(16).padStart(16, '0'));
    }
} else {
    throw new Error('expected reference, reference-bits or costs');
}
