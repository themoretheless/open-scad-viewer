from pathlib import Path
import json,re,struct,ast,math,hashlib
import sys
root=Path(sys.argv[1])
compare=json.loads((root/'comparison.json').read_text());assert compare['baseline']['changed']==[] and compare['baseline']['cases']==43
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
def number(v):return struct.unpack('>d',bytes.fromhex(v[1]))[0]
def diff(a,b,path='',result=None):
 if result is None:result=[]
 if type(a)!=type(b):result.append((path,'structure',a,b))
 elif isinstance(a,tuple):
  if a!=b:result.append((path,'number',number(a),number(b),a[1],b[1]))
 elif isinstance(a,dict):
  for k in sorted(a.keys()|b.keys()):
   if k not in a or k not in b:result.append((path+'/'+k,'field',a.get(k),b.get(k)))
   else:diff(a[k],b[k],path+'/'+k,result)
 elif isinstance(a,list):
  assert len(a)==len(b),(path,len(a),len(b))
  for i,(x,y) in enumerate(zip(a,b)):diff(x,y,path+f'/{i}',result)
 elif a!=b:result.append((path,'metadata',a,b))
 return result
report={'baselineSource':'2d63a3bec114041daf4cec8a789a882585d18646','currentSource':'3c19575792ef5002fe9fb85357bd0b36075caab5','run':38591445273,'baselineAll43HashesReproduced':True,'cases':{}}
for name in compare['current']['changed']:
 a=(root/'baseline'/name).read_text();b=(root/'current'/name).read_text()
 if a.startswith('{'):
  changes=diff(decode(a),decode(b));numeric=[x for x in changes if x[1]=='number'];other=[x for x in changes if x[1]!='number']
  if name.startswith('nurbs.intersect'):
   assert not numeric and all(x[1] in ['field','metadata'] for x in other)
   expected=['/coverage/certified','/coverage/complete','/coverage/searchComplete','/rounding']
   assert all(x[0] in expected for x in other),other
  else:
   assert not other,other
   assert all(x[0].startswith('/faces/') for x in numeric)
  report['cases'][name]={'numericChanges':len(numeric),'metadataChanges':other,'maxAbsoluteNumericDelta':max((abs(x[2]-x[3]) for x in numeric),default=0.),'sampleNumericChanges':numeric[:8]}
 else:
  tokens=lambda s:re.findall(r'[-+]?(?:\d+\.\d*|\.\d+|\d+)(?:e[-+]?\d+)?',s)
  aa,bb=tokens(a),tokens(b);assert len(aa)==len(bb)
  normalized=b.replace('reason: Tolerance','reason: "tolerance"').replace('reason: Separated','reason: "separated"')
  pattern=r'[-+]?(?:\d+\.\d*|\.\d+|\d+)(?:e[-+]?\d+)?'
  assert re.sub(pattern,'<numeric>',a)==re.sub(pattern,'<numeric>',normalized)
  changes=[(i,x,y,abs(float(x)-float(y))) for i,(x,y) in enumerate(zip(aa,bb)) if x!=y]
  assert len(changes)<=1 and all(v[3]<=2*math.ulp(float(v[1])) for v in changes)
  for text in [a,b]:
   pts=ast.literal_eval(text.split('points: ',1)[1].split(', point_enclosures:',1)[0]);bounds=ast.literal_eval(text.split('point_enclosures: ',1)[1].split(', converged:',1)[0])
   assert all(lo<=x<=hi for point,box in zip(pts,bounds) for x,(lo,hi) in zip(point,box))
  report['cases'][name]={'numericChanges':changes,'allWitnessesInsideUnchangedEnclosures':True,'reasonChange':'string diagnostic to native DistanceStopReason enum'}
(root/'verified-analysis.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
