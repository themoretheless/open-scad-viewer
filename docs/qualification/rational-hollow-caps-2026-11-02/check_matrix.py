import json,re,sys,itertools
from pathlib import Path
text=Path(sys.argv[1]).read_text()
rows=[]
for line in text.splitlines():
 m=re.match(r"(rational-hollow|spatial) capped authored=(true|false) arc=(true|false) affine=(true|false)(?: rational=(true|false))?: OriginalClassifiedCappedMaterialWork \{ cells: (\d+), exact_work: (\d+), products: (\d+), material_pairs: (\d+), material_cells: (\d+), material_domain_cells: (\d+) \}",line)
 if m:
  family,a,b,c,r,*work=m.groups()
  rows.append(dict(family=family,authored=a=='true',arc=b=='true',affine=c=='true',rational=r=='true' if r else True,work=dict(zip(['cells','exact_work','products','material_pairs','material_cells','material_domain_cells'],map(int,work)))))
keys=[(r['family'],r['authored'],r['arc'],r['affine'],r['rational']) for r in rows]
expected={('rational-hollow',a,b,c,True) for a,b,c in itertools.product([False,True],repeat=3)}
expected|={('spatial',a,b,c,False) for a,b,c in itertools.product([False,True],repeat=3)}
expected|={('spatial',a,False,True,True) for a in [False,True]}
assert len(keys)==len(set(keys))==18 and set(keys)==expected,(keys,expected-set(keys))
print(json.dumps(dict(distinctCases=len(keys),newRationalHollowCases=8,legacyCases=10,cases=rows),indent=2))
