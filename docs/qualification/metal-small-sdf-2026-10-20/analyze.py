#!/usr/bin/env python3
import csv, random, statistics, math, argparse
from pathlib import Path
p=argparse.ArgumentParser();p.add_argument('--prefix',default='paired');a=p.parse_args()
root=Path(__file__).resolve().parent
rng=random.Random(712)
out=[]
for run in range(1,4):
    groups={}
    for r in csv.DictReader((root/f'{a.prefix}-{run}.csv').open()):
        groups.setdefault((r['case'],r['field'],int(r['cells'])),[]).append(r)
    for (case,rows,cols),data in groups.items():
        logs=[math.log(float(r['a_ns'])/float(r['b_ns'])) for r in data]
        boot=sorted(math.exp(statistics.median(rng.choices(logs,k=len(logs)))) for _ in range(4000))
        v=[run,case,rows,cols,math.exp(statistics.median(logs)),boot[100],boot[3899],statistics.median(float(r['a_ns']) for r in data),statistics.median(float(r['b_ns']) for r in data)]
        out.append(v);print(run,case,rows,cols,*(round(x,3) for x in v[4:7]))
with (root/f'{a.prefix}-summary.csv').open('w') as f:
    w=csv.writer(f);w.writerow(['run','case','field','cells','speedup','ci_low','ci_high','a_ns','b_ns']);w.writerows(out)
