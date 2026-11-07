"""Verify DDA against the DFS GPU output and independent flat CPU oracle."""
from pathlib import Path
import math
import struct
import sys

root = Path(__file__).resolve().parent
run = root / sys.argv[1]
def unpack(path, fmt):
    return list(struct.iter_unpack(fmt, path.read_bytes()))

def row(values):
    order = sorted(values)
    return '/'.join(f'{order[math.ceil(p*len(order))-1]:.3f}' for p in (.5,.95,.99,1))

expected = root/'expected.bin' if run.name == 'dda-run' else run/'expected.bin'
oracle = unpack(expected, '<6f')
dfs = unpack(run/'tree.bin', '<4f')
dda = unpack(run/'dda.bin', '<4f')
assert len(oracle) == len(dfs) == len(dda) and oracle
print(f'{len(oracle)} rays; p50/p95/p99/max')
for values, label in ((dfs,'DFS'),(dda,'DDA')):
    err = [abs(a[0]-b[0]) for a,b in zip(oracle,values)]
    print(f'{label} flat CPU absolute error (percentage points): {row([e*100 for e in err])}')
    print(f'{label} texture loads: {row([v[1] for v in values])}; tested nodes: {row([v[2] for v in values])}')
    worst = max(range(len(err)), key=err.__getitem__)
    assert err[worst] < 0.00005, (oracle[worst],values[worst])
print(f'DFS/DDA worst difference (percentage points): {max(abs(a[0]-b[0])*100 for a,b in zip(dfs,dda)):.6f}')

ticks = unpack(run/'timestamps.bin', '<8Q')
assert all(0 < a <= b < 2**64-1 for tick in ticks for a,b in zip(tick[::2],tick[1::2]))
period = float((run/'metal.log').read_text().split('timestamp period ')[1].split(' ns')[0])
# The first six rows contain one full order permutation as warm-up.
deltas = [[(tick[2*i+1]-tick[2*i])*period/1e3 for tick in ticks[6:]] for i in range(4)]
for label, values in zip(('mips','legacy','DFS','DDA'),deltas):
    print(f'{label} pass (µs): {row(values)}')
print(f'DDA + mips (µs): {row([a+b for a,b in zip(deltas[0],deltas[3])])}')
print(f'DDA - DFS paired (µs): {row([a-b for a,b in zip(deltas[3],deltas[2])])}')
