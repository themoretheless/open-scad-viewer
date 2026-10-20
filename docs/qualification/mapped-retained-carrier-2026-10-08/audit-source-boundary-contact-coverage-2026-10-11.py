import json,pathlib,hashlib,math
p=pathlib.Path('docs/qualification/mapped-retained-carrier-2026-10-08')
paths=[p/'alternate-owned-path-dispatch-native-2026-10-11.json',p/'source-boundary-point-band-dispatch-native-2026-10-11.json']
a,b=[json.loads(x.read_text()) for x in paths]
assert a['request']==b['request']
def boxes(xs):
 result=[tuple(tuple(d) for rect in x for d in rect) for x in xs]
 assert all(len(x)==4 and all(len(d)==2 and all(math.isfinite(v) for v in d) and d[0]<d[1] for d in x) for x in result), 'Audit requires finite nondegenerate boxes'
 return result
def subtract(x,y):
 if any(min(h,k)<=max(l,j) for (l,h),(j,k) in zip(x,y)):return [x]
 core=list(x);out=[]
 for i,((l,h),(j,k)) in enumerate(zip(x,y)):
  lo,hi=max(l,j),min(h,k)
  if l<lo:
   z=core.copy();z[i]=(l,lo);out.append(tuple(z))
  if hi<h:
   z=core.copy();z[i]=(hi,h);out.append(tuple(z))
  core[i]=(lo,hi)
 return out
def covered(x,ys):
 remain=[x]
 for y in ys:
  remain=[z for r in remain for z in subtract(r,y)]
  if not remain:return True
  if len(remain)>100000:raise RuntimeError('audit subdivision limit')
 return False
# Exact binary64 endpoint comparisons; no rounded volumes or sampled points.
results=[]
ca=a['response']['shell']['inspection']['contacts']['pairs'];cb=b['response']['shell']['inspection']['contacts']['pairs'];assert len(ca)==len(cb)
for i,(old,new) in enumerate(zip(ca,cb)):
 assert old['faces']==new['faces']
 oldproof=boxes(old.get('separatedProducts',[])+old.get('outsideProducts',[]));newproof=boxes(new.get('separatedProducts',[])+new.get('outsideProducts',[]))
 pendingold=boxes(old.get('pendingProducts',[]));pendingnew=boxes(new.get('pendingProducts',[]))
 if oldproof==newproof and pendingold==pendingnew:continue
 results.append({'pairIndex':i,'oldProofNotCovered':sum(not covered(x,newproof) for x in oldproof),'newPendingNotCovered':sum(not covered(x,pendingold) for x in pendingnew)})
r={'sameRequest':True,'geometryQualified':False,'scope':'Closed nondegenerate parameter boxes, exact endpoint partition; checks coverage of reported contact products only. No whole-solid admission.','sourceSha256':{x.name:hashlib.sha256(x.read_bytes()).hexdigest() for x in paths},'pairs':results}
r['oldProofPreserved']=all(x['oldProofNotCovered']==0 for x in results);r['newPendingContained']=all(x['newPendingNotCovered']==0 for x in results)
(p/'source-boundary-point-band-contact-coverage-2026-10-11.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r))
