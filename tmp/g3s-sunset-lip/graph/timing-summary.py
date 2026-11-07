from pathlib import Path
import json
import math
import sys

path=Path(sys.argv[1]) if len(sys.argv)>1 else Path(__file__).resolve().parent
rounds = {mode:[json.loads(line) for line in (path/f'{mode}.jsonl').read_text().splitlines()]
          for mode in ('taps1','cells1','cells2','taps2')}

def summary(values):
    values=sorted(values)
    return '/'.join(f'{values[math.ceil(q*len(values))-1]:.3f}' for q in (.5,.95,.99))

for mode, rows in rounds.items():
    assert len(rows)==80
    cooked = all(row.get('cooked_generation') is not None for row in rows)
    if cooked:
        assert all(row['cooked_submitted']==rows[0]['cooked_submitted']+index for index,row in enumerate(rows))
    print(mode, 'cooked submission counter' if cooked else 'contribution counter absent')
    valid=sum(row['gpu_ns'] is not None and not row['timing_fault'] for row in rows)
    print(mode, valid, 'of 80 valid timestamp rows')
    assert valid == 80
    for begin in (0,20,40,60):
        sample=rows[begin+3:begin+20]
        print(f"  {sample[0]['hours']:.5f} h GPU total {summary([sum(r['gpu_ns'])/1e6 for r in sample])} ms "
              f"far {summary([r['gpu_ns'][1]/1e6 for r in sample])} forward "
              f"{summary([r['gpu_ns'][3]/1e6 for r in sample])}; render CPU "
              f"{summary([r['render_cpu_ns']/1e6 for r in sample])} ms")
