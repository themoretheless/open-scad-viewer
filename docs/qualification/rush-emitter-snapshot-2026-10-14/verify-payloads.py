from pathlib import Path
import json,struct,hashlib,re,difflib,collections
import sys
root=Path(sys.argv[1])
def fnv(b):
 h=0xcbf29ce484222325
 for v in b:h=((h^v)*0x100000001b3)&((1<<64)-1)
 return f'{h:016x}'
def rows(name,file):return [line.split('\t') for line in (root/name/file).read_text().splitlines()]
def decode(s):return json.loads(s,parse_float=lambda s:('f64',struct.pack('>d',float(s)).hex()))
changes=[]
def diff(a,b,path):
 if type(a)!=type(b):changes.append((path,'type',a,b));return
 if isinstance(a,dict):
  for k in sorted(a.keys()|b.keys()):
   if k not in a or k not in b:changes.append((path+'/'+k,'field',a.get(k),b.get(k)))
   else:diff(a[k],b[k],path+'/'+k)
 elif isinstance(a,list):
  if len(a)!=len(b):changes.append((path,'length',len(a),len(b)))
  else:
   for i,(x,y) in enumerate(zip(a,b)):diff(x,y,path+'/'+str(i))
 elif a!=b:changes.append((path,'number' if isinstance(a,(int,float,tuple)) else 'text',a,b))
for name in ['baseline','current']:
 for label,kind,size,hash in rows(name,'actual-snapshot.tsv'):
  b=(root/name/label.replace('/','__')).read_bytes();assert len(b)==int(size) and fnv(b)==hash
assert rows('baseline','actual-snapshot.tsv')==rows('baseline','original-snapshot.tsv')
old=rows('baseline','actual-snapshot.tsv');new=rows('current','actual-snapshot.tsv');assert len(old)==len(new)==81
for a,b in zip(old,new):
 assert a[0].replace('MODELGRAPH','RUSH_GRAPH')==b[0],(a[0],b[0])
 x=(root/'baseline'/a[0].replace('/','__')).read_text();y=(root/'current'/b[0].replace('/','__')).read_text();
 assert x.split(':',1)[0]==y.split(':',1)[0],b[0]
 diff(decode(x.split(':',1)[1]),decode(y.split(':',1)[1]),b[0])
c=collections.Counter(v[1] for v in changes)
assert set(c)<= {'number','text'},dict(c)
f=lambda v:struct.unpack('>d',bytes.fromhex(v[1]))[0]
nums=[x for x in changes if x[1]=='number']
allowed={'RUSH_GRAPH_SKETCH_EXAMPLE','sketch_parallel','sketch_perpendicular','sketch_equal_length','sketch_coincident'}
for path,kind,a,b in nums:
    if 'ASSEMBLY_EXAMPLE' in path: assert f(a)==f(b)==0.
    else: assert path.split('/')[1] in allowed and '/sketch_solutions/' in path and abs(f(a)-f(b))<1e-6
num_pattern=r'[-+]?(?:\d+\.\d*|\.\d+|\d+)(?:e[-+]?\d+)?'
for path,kind,a,b in changes:
    if kind!='text':continue
    a=a.replace('modelgraph/1','rush/ir-1')
    if a==b:continue
    assert path.endswith('/source'),path
    strip=lambda s:re.sub(r'//[^\n]*','',s)
    if path=='extra/planetary_spinner/source':assert strip(a)==strip(b)
    else:
        assert path.split('/')[1] in allowed,path
        aa,bb=re.findall(num_pattern,a),re.findall(num_pattern,b);assert len(aa)==len(bb)
        assert re.sub(num_pattern,'<numeric>',a)==re.sub(num_pattern,'<numeric>',b)
        assert all(abs(float(x)-float(y))<1e-6 for x,y in zip(aa,bb))
solutions=0
for p in (root/'current').glob('parity__*'):
    text=p.read_text()
    if not text.startswith('ok:'):continue
    for solution in json.loads(text.split(':',1)[1]).get('sketch_solutions',[]):
        solutions+=1
        assert solution['maximum_residual_mm']<=solution['tolerance_mm']
        assert all(c['satisfied'] for c in solution['constraints'])
report={'cases':81,'baselineHashesReproduced':81,'currentStatuses':dict(collections.Counter(row[1] for row in new)),'statusesPreserved':True,'changedRows':sum(a!=b for a,b in zip(rows('current','original-snapshot.tsv'),new)),'numericLeafChanges':len(nums),'maxObservedNumericDelta':max(abs(f(x[2])-f(x[3])) for x in nums),'unchangedSketchToleranceMm':1e-6,'sketchSolutionsWithinDeclaredTolerance':solutions,'classificationAndStructureChanged':False,'changes':changes}
(root/'verified-analysis.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({k:v for k,v in report.items() if k!='changes'},indent=2))
