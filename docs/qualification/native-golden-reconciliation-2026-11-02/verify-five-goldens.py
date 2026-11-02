from pathlib import Path
import json,struct,math,re,ast,hashlib,copy
Q=Path(__file__).parent
x=json.loads((Q/'golden-structured-payloads-20261102.json').read_text())
def num(v):return struct.unpack('>d',bytes.fromhex(v[1]))[0]
def changes(a,b,path=''):
 if a==b:return []
 if isinstance(a,list) and len(a)==2 and a[0]=='f64':return [(path,num(a),num(b))]
 assert type(a)==type(b),(path,a,b)
 if isinstance(a,dict):
  assert a.keys()==b.keys(),path
  return sum((changes(a[k],b[k],path+'/'+k) for k in a),[])
 if isinstance(a,list):
  assert len(a)==len(b),path
  return sum((changes(v,w,path+'/'+str(i)) for i,(v,w) in enumerate(zip(a,b))),[])
 raise AssertionError((path,a,b))
class Reader:
 def __init__(self,s):self.s=s;self.i=0
 def text(self,tag):
  assert self.s[self.i]==tag;self.i+=1;j=self.s.index(':',self.i);n=int(self.s[self.i:j]);self.i=j+1;v=self.s[self.i:self.i+n];self.i+=n;return v
 def value(self):
  c=self.s[self.i]
  if c in 'ntf':self.i+=1;return {'n':None,'t':True,'f':False}[c]
  if c=='s':return self.text('s')
  if c=='d':self.i+=1;v=('f64',self.s[self.i:self.i+16]);self.i+=16;return v
  if c in 'ui':
   self.i+=1;m=re.match(r'-?\d+',self.s[self.i:]);assert m;self.i+=len(m[0]);return int(m[0])
  if c=='[':
   self.i+=1;v=[]
   while self.s[self.i]!=']':v.append(self.value())
   self.i+=1;return v
  if c=='{':
   self.i+=1;v={}
   while self.s[self.i]!='}':k=self.text('k');v[k]=self.value()
   self.i+=1;return v
  raise ValueError((self.i,c))

def decode(s):
 r=Reader(s);v=r.value();assert r.i==len(s);return v
for case, variants in x.items():
 for variant, value in variants.items():
  raw=(Q/f'golden-{variant}-linux'/f'{case}.payload').read_text()
  assert json.loads(json.dumps(decode(raw)))==value, (case, variant)

a,b=copy.deepcopy(x['nurbs.intersect_surface_surface']['baseline']),x['nurbs.intersect_surface_surface']['current']
assert b['unresolved'][1:]==a['unresolved']
assert b['unresolved'][0]=={'classification':'unproven_reduction','parameterBox':[['f64',v] for v in ['0000000000000000','3ff0000000000000']*4],'reason':'conditioning_boundary'}
a['unresolved'].insert(0,b['unresolved'][0])
expected=[('branchGraph/certificate/geometryCoverageComplete',None,False),('branchGraph/certificate/method','4d-Bernstein-Krawczyk-continuation','native_components_without_owned_graph_proof'),('branchGraph/certificate/oneToOneJoins',True,False),('coverage/candidateSeeds',None,0),('coverage/proofScope',None,'unproven'),('coverage/topologyProven',None,False),('coverage/method','4D-Bernstein-hull-exclusion-Krawczyk-continuation','candidate_subdivision_and_continuation'),('rounding','binary64-nextafter-outward','candidate_binary64'),('topologyAuthority/reason','unresolved_or_resource_or_conditioning','coverage_unproven'),('uvArrangement/kind','rational_curved_dcel','candidate_uv_traces')]
for path,old,new in expected:
 aa,bb=a,b
 parts=path.split('/')
 for k in parts[:-1]:aa,bb=aa[k],bb[k]
 k=parts[-1]
 if old is None:assert k not in aa
 else:assert aa[k]==old
 assert bb[k]==new;aa[k]=new
assert a==b
assert not b['booleanMutationAuthority'] and not b['branchGraph']['permitsTopologyAuthorship'] and not b['topologyAuthority']['granted']
report={'surfaceIntersection':{'allOldUnresolvedPreserved':1452,'newFullDomainUnresolved':1,'metadataChanges':expected,'allOtherValuesBitIdentical':True,'topologyAuthority':False}}
for variant,z in x['sketch.solve'].items():
 pts=[[num(v) for v in p] for p in z['sketch']['points']];r=num(z['sketch']['circles'][0]['radius'])
 a,b,c,d,e=pts;u=[b[i]-a[i] for i in range(2)];v=[c[i]-a[i] for i in range(2)]
 residuals=[[a[0],a[1]],[b[0]-10,b[1]],[r-2],[abs(u[0]*v[1]-u[1]*v[0])/math.hypot(*u)-r],[math.dist(c,d)-5.5],[0.,0.],[math.dist(d,e)-6.25]]
 reported=[[num(v) for v in row] for row in z['residuals']]
 error=max(abs(a-b) for row,rr in zip(residuals,reported) for a,b in zip(row,rr))
 assert error<4e-15,error
 actual=max(abs(v) for row in residuals for v in row)
 assert actual<1e-7 and abs(actual-num(z['maxResidual']))<4e-15
 assert z['convergenceCertified'] is False and z['status']=='underconstrained' and z['degreesOfFreedom']==3 and z['iterations']==3
 report['sketch_'+variant]={'independentMaxResidual':actual,'reportedResidualMaxDifference':error,'constraintsSatisfiedWithinRequestedTolerance':True,'globalCertificate':False}
diffs=changes(x['sketch.solve']['baseline'],x['sketch.solve']['current'])
assert all(p.startswith(('/sketch/points/','/sketch/circles/0/radius','/residuals/','/maxResidual')) for p,_,_ in diffs)
report['sketchNumericChanges']=diffs
pattern=r'[-+]?(?:\d+\.\d*|\.\d+|\d+)(?:e[-+]?\d+)?'
for name in ['nurbs.surface_distance','nurbs.surface_offset.1','nurbs.surface_offset.2']:
 a=(Q/'golden-baseline-linux'/f'{name}.payload').read_text();b=(Q/'golden-current-linux'/f'{name}.payload').read_text()
 assert re.sub(pattern,'#',a)==re.sub(pattern,'#',b)
 aa,bb=re.findall(pattern,a),re.findall(pattern,b)
 assert len(aa)==len(bb)
 diffs=[(i,float(a),float(b)) for i,(a,b) in enumerate(zip(aa,bb)) if a!=b]
 assert all(math.isfinite(float(v)) for v in aa+bb)
 if name.endswith('distance'):
  assert diffs==[(0,3.9501702380625714,3.9501702380625705)]
  pts=ast.literal_eval(b.split('points: ')[1].split(', point_enclosures:')[0]);boxes=ast.literal_eval(b.split('point_enclosures: ')[1].split(', converged:')[0])
  assert all(lo<=v<=hi for p,box in zip(pts,boxes) for v,(lo,hi) in zip(p,box))
  assert math.dist(*pts)<=float(bb[1])
 report[name]={'changedNumbers':diffs,'allOtherTextIdentical':True}
report['offsetIndependentOracle']=json.loads((Q/'golden-offset-independent-oracle-20261102.json').read_text())
(Q/'golden-five-reconciliation-20261102.json').write_text(json.dumps(report,indent=2)+'\n')
print('PASS: five differences checked; old SS values intact, conservative distance interval, independent offset jets and sketch residuals.')
