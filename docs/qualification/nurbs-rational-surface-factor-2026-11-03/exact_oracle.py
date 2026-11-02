"""Independent full tensor identities from actual Rust-returned binary64 data."""
import ast,json,sys,copy,hashlib
from pathlib import Path
p=Path('docs/qualification/nurbs-exact-surface-knot-insertion-2026-11-02/exact_oracle.py')
tree=ast.parse(p.read_text());tree.body=[n for n in tree.body if isinstance(n,(ast.Import,ast.ImportFrom,ast.FunctionDef))];scope={};exec(compile(tree,str(p),'exec'),scope)
def rows(path):
 out=[];decoder=json.JSONDecoder()
 for line in Path(path).read_text().splitlines():
  if 'surface-factor-oracle=' in line:out.append(decoder.raw_decode(line.split('surface-factor-oracle=',1)[1])[0])
 return sorted(out,key=lambda r:json.dumps(r,sort_keys=True))
a,b=rows(sys.argv[1]),rows(sys.argv[2]);assert len(a)==len(b)==6 and a==b
identity=scope['identity'];assert all(identity(r['source'],r['output']) for r in a)
r=next(r for r in a if r['source']['degreeU']==2 and r['source']['degreeV']==1 and r['source']['weights']==[[1.,3.],[2.,6.],[4.,12.]])
wrong=copy.deepcopy(r['output'])
for row in wrong['weights']:row[1]*=2
assert not identity(r['source'],wrong)
wrong=copy.deepcopy(r['output']);wrong['controlPoints'][1][0][0]=__import__('math').nextafter(wrong['controlPoints'][1][0][0],float('inf'));assert not identity(r['source'],wrong)
wrong=copy.deepcopy(r['output']);wrong['knotsU']=[x+1 for x in wrong['knotsU']];assert not identity(r['source'],wrong)
source=copy.deepcopy(r['source']);source['controlPoints']=[[[0.,0.,0.],[0.,3.,3.]] for _ in range(3)];source['weights']=[[1.,4.],[2.,2.],[4.,1.]]
wrong=copy.deepcopy(r['output']);wrong['controlPoints']=[[[0.,0.,0.],[0.,3.,3.]] for _ in range(2)];wrong['weights']=[[1.,1.],[1.,1.]];assert not identity(source,wrong)
print(json.dumps({'acceptedDefinitions':len(a),'negativeControls':4,'failed':0,'definitionsSha256':hashlib.sha256(json.dumps(a,sort_keys=True).encode()).hexdigest(),'independentOracleSourceSha256':hashlib.sha256(p.read_bytes()).hexdigest(),'scope':'Exact bivariate homogeneous polynomial cross-products on every original/output tensor knot cell, actual binary64 output. Degree25 and untouched periodic support included. Detects per-row gauges, oneULP output damage, clock shift and independent-row factor cancellation. No sampling, topology, current WASM or compiler claim.'},indent=2))
