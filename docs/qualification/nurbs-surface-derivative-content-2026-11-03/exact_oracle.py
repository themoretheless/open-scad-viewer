"""Fresh source-field proofs; compare geometry, not obsolete control encodings."""
from pathlib import Path
from collections import Counter,defaultdict
from math import nextafter,inf
import copy,json,runpy,sys,hashlib
BASE=Path('docs/qualification/nurbs-exact-surface-derivative-2026-11-03')
original=runpy.run_path(str(BASE/'exact_oracle.py'));verify=original['verify'];rows=original['rows']

def new_rows(path):
    decoder=json.JSONDecoder()
    return [decoder.raw_decode(line.split('exact-surface-content-oracle=',1)[1])[0] for line in Path(path).read_text().splitlines() if 'exact-surface-content-oracle=' in line]

def key(r):return json.dumps({'axis':r['axis'],'source':r['source']},sort_keys=True)

def groups(records):
    result=defaultdict(list)
    for r in records:result[key(r)].append(r)
    return result

if __name__=='__main__':
    current,previous=rows(sys.argv[1]),rows(sys.argv[2]);fresh=new_rows(sys.argv[1]);assert len(current)==len(previous)==17 and len(fresh)==4
    frozen=json.loads((BASE/'receipt.json').read_text())['archives']['native245-default.log.gz']['decodedSha256']
    assert hashlib.sha256(Path(sys.argv[2]).read_bytes()).hexdigest()==frozen
    assert Counter(map(key,current))==Counter(map(key,previous))
    assert all(verify(r) for r in previous+current+fresh)
    old,new=groups(previous),groups(current);changed_records=changed_patches=0
    for k in old:
        for a,b in zip(old[k],new[k]):
            assert len(a['patches'])==len(b['patches'])
            changed_records+=a['patches']!=b['patches']
            changed_patches+=sum(x!=y for x,y in zip(a['patches'],b['patches']))
    seed=next(r for r in fresh if r['source']['degreeU']==6 and r['source']['degreeV']==1);bad=[]
    r=copy.deepcopy(seed);r['patches'][0]['controlPoints'][0][0][0]=nextafter(r['patches'][0]['controlPoints'][0][0][0],inf);bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['weights'][1]=[2*w for w in r['patches'][0]['weights'][1]];bad.append(r)
    r=copy.deepcopy(seed);r['axis']='v';bad.append(r)
    r=copy.deepcopy(seed);r['source']['knotsU']=[x*2 for x in r['source']['knotsU']];bad.append(r)
    r=copy.deepcopy(seed);r['source']['controlPoints'][0][1][1]+=1;bad.append(r)
    r=copy.deepcopy(seed);r['source']['weights'][0][0]*=2;bad.append(r)
    r=copy.deepcopy(seed);r['patches']=[];bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['periodicU']=True;bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['knotsV'][-1]+=1;bad.append(r)
    r=copy.deepcopy(seed);r['patches'][0]['controlPoints'][1][0][0]=0.;bad.append(r)
    assert all(not verify(r) for r in bad)
    gauge=copy.deepcopy(seed);gauge['patches'][0]['weights']=[[2*w for w in row] for row in gauge['patches'][0]['weights']];assert verify(gauge)
    assert changed_records>0 and changed_patches>0
    print(json.dumps({'existingSourceRecords':len(current),'newSourceRecords':len(fresh),'currentOutputPatches':sum(len(r['patches']) for r in current+fresh),'changedExistingRepresentations':changed_records,'changedExistingPatchEncodings':changed_patches,'negativeControls':len(bad),'positiveGlobalGaugeControl':True,'currentDefinitionsSha256':hashlib.sha256(json.dumps(current+fresh,sort_keys=True).encode()).hexdigest(),'verifierSourceSha256':hashlib.sha256((BASE/'exact_oracle.py').read_bytes()).hexdigest(),'scope':'Every prior and new actual output independently satisfies full bivariate original-source partial identity, original UV clocks and cell cover. Source/axis multiset unchanged; binary net representations intentionally may shrink. No old topology/certificate or installed artifact authority transferred.'},indent=2))
