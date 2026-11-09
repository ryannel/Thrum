"""Independent zstd CLI decode plus reversible XOR; verify original writer hashes."""
import hashlib
import subprocess
from pathlib import Path
from t2_writer_probe import FRAME_BYTES


def decoded_frames(path, encoding, expected):
    if len(expected) > 1024:
        raise ValueError('capture record limit exceeded')
    process = None
    if encoding == 'ZstdDelta':
        process = subprocess.Popen(['zstd', '-d', '-q', '-c', str(path)], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        stream = process.stdout
    elif encoding == 'Raw':
        stream = Path(path).open('rb')
    else:
        raise ValueError('unknown lossless spool encoding')
    previous = 0
    try:
        for digest in expected:
            parts, remaining = [], FRAME_BYTES
            while remaining:
                block = stream.read(remaining)
                if not block:
                    raise ValueError('truncated lossless spool')
                parts.append(block)
                remaining -= len(block)
            raw = b''.join(parts)
            if encoding == 'ZstdDelta':
                previous ^= int.from_bytes(raw, 'little')
                raw = previous.to_bytes(FRAME_BYTES, 'little')
            if hashlib.sha256(raw).hexdigest() != digest:
                raise ValueError('decoded spool differs from original writer RGBA hash')
            yield raw
        if stream.read(1):
            raise ValueError('extra decoded bytes')
        if process is not None and process.wait(timeout=15):
            raise ValueError('independent zstd decoder refused source')
    finally:
        stream.close()
        if process is not None and process.poll() is None:
            process.kill()
            process.wait()
        if process is not None:
            process.stderr.close()
