"""Compare Metal horizon results with the two CPU referees and retain raw GPU timestamps."""
from pathlib import Path
import math
import struct

p = Path(__file__).resolve().parent
unpack = lambda name, fmt: list(struct.iter_unpack(fmt, (p/name).read_bytes()))
exp = unpack('expected.bin', '<6f')
old = unpack('legacy.bin', '<4f')
new = unpack('tree.bin', '<4f')
assert len(exp) == len(old) == len(new) and len(exp)>0

def percentile(values, q):
    return sorted(values)[math.ceil(q*len(values))-1]

def stats(values):
    return tuple(round(percentile(values,q),6) for q in (.5,.95,.99,1))

def fmt(values):
    return '/'.join(str(x) for x in stats(values))

print('Rows are p50/p95/p99/max.')
for data,name in ((old,'legacy taps'),(new,'max hierarchy')):
    print(f'{name}: reads {fmt([row[1] for row in data])}')
    for label, idx in (('flat CPU', 0), ('exact triangles CPU', 1)):
        error = [a[0]-b[idx] for a,b in zip(data,exp)]
        print(f'  {label}: abs error pp {fmt([abs(e)*100 for e in error])}, '
              f'max leak pp {max(0,max(error))*100:.6f}')
print(f'hierarchy node tests {fmt([row[2] for row in new])}')
for a,b,c in zip(exp,old,new):
    if abs(a[2] - (17.4+35/120)) < 1e-5 and a[3] == -90 and a[5] == 434:
        print(f'witness {int(a[4])}/434: exact {a[1]:.6f}, flat {a[0]:.6f}, Metal {b[0]:.6f} -> {c[0]:.6f}')
rows = unpack('timestamps.bin', '<6Q')
assert all(0 < a <= b < 2**64-1 for row in rows for a,b in zip(row[::2],row[1::2]))
log = (p/'metal.log').read_text()
period = float(log.split('timestamp period ')[1].split(' ns')[0])
deltas = [[(row[2*i+1]-row[2*i])*period/1e3 for row in rows[2:]] for i in range(3)]
for label, data in zip(('mips','legacy','hierarchy'),deltas):
    print(f'{label} pass µs {fmt(data)}')
print(f'hierarchy plus reduction µs {fmt([x+y for x,y in zip(deltas[0],deltas[2])])}')
errors = [abs(a[0]-b[0]) for a,b in zip(exp,new)]
assert max(errors) < .0005, max(errors)
