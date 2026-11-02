"""Independent exact check of the authored failing reversal fixture.

Load only function/import definitions from the immutable earlier B-spline
Fraction oracle; do not execute its historical log-reading entry point.
"""
import ast,json,hashlib
from fractions import Fraction as F
from pathlib import Path
path=Path('docs/qualification/nurbs-exact-knot-insertion-2026-11-02/exact_oracle.py')
tree=ast.parse(path.read_text());tree.body=[n for n in tree.body if isinstance(n,(ast.Import,ast.ImportFrom,ast.FunctionDef))]
ns={};exec(compile(tree,str(path),'exec'),ns)

def source(k):return {'degree':2,'knots':[0.,0.,0.,k,1.,1.,1.],'controlPoints':[[0.,0.,z] for z in [0.,1.,3.,5.]],'weights':[1.,.75,1.5,1.]}
def normalized_formal_reverse(k):
 # Curve::reverse stores the rounded mirror. Formal reversal of that actual
 # curve uses exact opposite knots and exact affine domain normalization.
 actual=[0.,0.,0.,1.-k,1.,1.,1.]
 neg=[-F(x) for x in reversed(actual)];lo,hi=neg[2],neg[4]
 c=source(k);c['knots']=[(x-lo)/(hi-lo) for x in neg];return c
checks={
 'binary64-rounded-complement-sums-to-one':0.1+(1.-0.1)==1.,
 'exact-complement-has-nonzero-residual':F(.1)+F(1.-.1)!=1,
 'rounded-reversal-law-is-different':not ns['identity'](source(.1),normalized_formal_reverse(.1)),
 'dyadic-reversal-law-is-equal':ns['identity'](source(.25),normalized_formal_reverse(.25)),
}
assert all(checks.values())
print(json.dumps({'passed':len(checks),'failed':0,'checks':checks,'exactComplementResidual':str(F(.1)+F(1.-.1)-1),'oracleSourceSha256':hashlib.sha256(path.read_bytes()).hexdigest(),'scope':'Exact cellwise polynomial cross products for the authored regression fixture and dyadic positive control. No source-runtime, topology or whole-workspace claim.'},indent=2))
