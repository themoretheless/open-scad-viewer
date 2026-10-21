#!/usr/bin/env python3
"""Log-ratio paired bootstrap, fixed seed, all samples retained."""
import csv, random, statistics, math
from pathlib import Path
root=Path(__file__).resolve().parent
rng=random.Random(712)
rows=[]
for run in range(1,4):
    groups={}
    for r in csv.DictReader((root/f'paired-{run}.csv').open()):
        groups.setdefault((r['case'],int(r['bytes']),r['phase']),[]).append(r)
    for (case,size,phase),v in groups.items():
        log=[math.log(float(x['a_ns'])/float(x['b_ns'])) for x in v]
        boots=sorted(math.exp(statistics.median(rng.choices(log,k=len(log)))) for _ in range(4000))
        rows.append([run,case,size,phase,math.exp(statistics.median(log)),boots[100],boots[3899],statistics.median(float(x['a_ns']) for x in v),statistics.median(float(x['b_ns']) for x in v)])
with (root/'summary.csv').open('w') as f:
    w=csv.writer(f);w.writerow(['run','case','bytes','phase','speedup','ci_low','ci_high','a_ns','b_ns']);w.writerows(rows)
for size in [16,64,256,1024,4096]:
    for phase in ['cpu','full']:
        v=[r for r in rows if r[1]=='four_parts' and r[2]==size and r[3]==phase]
        print(size,phase,'; '.join(f'{r[4]:.3f} [{r[5]:.3f},{r[6]:.3f}]' for r in v))
