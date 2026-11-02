from pathlib import Path
from fractions import Fraction as F
import json
rows=[]
for line in Path(__file__).with_name('before-probe.log').read_text().splitlines():
    if line.startswith('source_knots='): knots=[F(x) for x in json.loads(line.split('=',1)[1])]
    elif line.startswith('source_controls='): controls=[[F(x) for x in row] for row in json.loads(line.split('=',1)[1])]
    elif line.startswith('delta='):
        c,k=controls,knots
        for degree in [3,2,1]:
            c=[[(b-a)*degree/(k[i+degree+1]-k[i+1]) for a,b in zip(c[i],c[i+1])] for i in range(len(c)-1)]
            k=k[1:-1]
        rows.append((line,[b-a for a,b in zip(c[0],c[1])]))
assert rows[0][1]==[F(0),F(0)]
assert 'delta=1e-10 reported_tolerance=0e0' in rows[1][0]
assert rows[1][1]==[F(0),F(56295,8796093022208)]
print('Verified nonzero exact third-derivative jump at removed knot, despite reported zero tolerance.')
