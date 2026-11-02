from decimal import Decimal as D, getcontext
from pathlib import Path
import re,ast,json,math
getcontext().prec=90
Q=Path(__file__).parent
F=lambda x:D.from_float(float(x))
K=list(map(F,[0,0,0,0,.4,1,1,1,1]))
P=[[0,0,0],[1,2,.3],[2.5,-1,.1],[3.7,1.4,-.5],[5,.2,0]]
W=list(map(F,[1,.8,1.3,1,1]))
net=[[[F(x) for x in p],[F(x+d) for x,d in zip(p,[.2,.3,2.])]] for p in P]
def basis(i,p,u,r):
 if r>p:return D(0)
 if r:
  a=K[i+p]-K[i];b=K[i+p+1]-K[i+1]
  return (D(p)*basis(i,p-1,u,r-1)/a if a else D(0))-(D(p)*basis(i+1,p-1,u,r-1)/b if b else D(0))
 if p==0:return D(int(K[i]<=u<K[i+1]))
 a=K[i+p]-K[i];b=K[i+p+1]-K[i+1]
 return ((u-K[i])*basis(i,p-1,u,0)/a if a else D(0))+((K[i+p+1]-u)*basis(i+1,p-1,u,0)/b if b else D(0))
def cross(a,b):return [a[(i+1)%3]*b[(i+2)%3]-a[(i+2)%3]*b[(i+1)%3] for i in range(3)]
def dot(a,b):return sum(x*y for x,y in zip(a,b))
def add(a,b):return [x+y for x,y in zip(a,b)]
def oracle(uv):
 u,v=map(F,uv)
 H={}
 for r,s in [(0,0),(1,0),(0,1),(2,0),(1,1),(0,2)]:
  H[r,s]=[sum(basis(i,3,u,r)*([1-v,v][j] if s==0 else [-D(1),D(1)][j] if s==1 else D(0))*W[i]*(net[i][j][k] if k<3 else D(1)) for i in range(5) for j in range(2)) for k in range(4)]
 w=H[0,0][3];p=[x/w for x in H[0,0][:3]]
 du=[(H[1,0][k]-p[k]*H[1,0][3])/w for k in range(3)]
 dv=[(H[0,1][k]-p[k]*H[0,1][3])/w for k in range(3)]
 duu=[(H[2,0][k]-2*du[k]*H[1,0][3]-p[k]*H[2,0][3])/w for k in range(3)]
 duv=[(H[1,1][k]-du[k]*H[0,1][3]-dv[k]*H[1,0][3]-p[k]*H[1,1][3])/w for k in range(3)]
 dvv=[(H[0,2][k]-2*dv[k]*H[0,1][3]-p[k]*H[0,2][3])/w for k in range(3)]
 raw=cross(du,dv);length=dot(raw,raw).sqrt();n=[x/length for x in raw]
 def nd(d):
  parallel=dot(n,d)
  return [(d[k]-n[k]*parallel)/length for k in range(3)]
 nu=nd(add(cross(duu,dv),cross(du,duv)));nv=nd(add(cross(duv,dv),cross(du,dvv)))
 return {'point':[p[k]+D('.25')*n[k] for k in range(3)],'du':[du[k]+D('.25')*nu[k] for k in range(3)],'dv':[dv[k]+D('.25')*nv[k] for k in range(3)],'source_unit_normal':n}
report={'precisionDecimalDigits':90,'inputModel':'Exact binary64 nominal controls, weights, knots, parameters; extruded control additions rounded as binary64 before conversion.','checks':[]}
for i,uv in enumerate([[.1,.2],[.55,.5],[.93,.81]]):
 ref=oracle(uv)
 for variant in ['baseline','current']:
  text=(Q/f'golden-{variant}-linux'/f'nurbs.surface_offset.{i}.payload').read_text()
  fields={key:ast.literal_eval(value) for key,value in re.findall(r'(point|du|dv|source_unit_normal): (\[[^\]]+\])',text)}
  assert set(fields)==set(ref)
  errors={key:[float(abs(F(x)-y)) for x,y in zip(fields[key],ref[key])] for key in ref}
  # Independent absolute check, well below the application geometry tolerance.
  assert max(x for a in errors.values() for x in a)<D('5e-13'),errors
  report['checks'].append({'sample':i,'variant':variant,'reference':{k:[str(x) for x in v] for k,v in ref.items()},'absoluteErrors':errors})
(Q/'golden-offset-independent-oracle-20261102.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'samples':3,'variants':2,'componentsChecked':72,'maxAbsoluteError':max(x for c in report['checks'] for a in c['absoluteErrors'].values() for x in a)},indent=2))
