#!/usr/bin/env python3
"""Independent sampled diagnostics of an UNADMITTED variable collar proposal."""
import hashlib,json,math,sys
from pathlib import Path
import occt_surface_jets as jets
root=Path(sys.argv[1]);reports=[]
for directory in sorted(root.glob('case-*')):
    path=directory/'proposal.json';p=json.loads(path.read_text())
    assert p['savedBodyAdmitted'] is False and p['refusalCode']=='BREP_END_COLLAR_UNPROVEN'
    skin=jets.surface(p['surface']);supports=[jets.surface(s) for s in p['supports']]
    contacts=[jets.curve(c) for c in p['contacts']];pcs=[jets.curve(c) for c in p['pcurves']];skin_pcs=[jets.curve(c) for c in p['skinPcurves']]
    r0,r1=p['radiiMm'];pad=p['padding'];full=[r0-pad*(r1-r0),r1+pad*(r1-r0)]
    position=normal=tensor=centers=0.;samples=0
    for i in range(65):
        t=i/64;data=[];ball=[]
        for side in range(2):
            c=contacts[side];world=c.Value(c.FirstParameter()+(c.LastParameter()-c.FirstParameter())*t).Coord()
            pc=pcs[side];uv=pc.Value(pc.FirstParameter()+(pc.LastParameter()-pc.FirstParameter())*t)
            a,n,k=jets.jets(supports[side],uv.X(),uv.Y())
            sp=skin_pcs[side];q=sp.Value(sp.FirstParameter()+(sp.LastParameter()-sp.FirstParameter())*t)
            b,n2,k2=jets.jets(skin,q.X(),q.Y());sign=1 if jets.dot(n,n2)>=0 else -1
            position=max(position,math.dist(world,a),math.dist(world,b))
            normal=max(normal,math.dist(n,[sign*v for v in n2]))
            tensor=max(tensor,max(abs(k[x][y]-sign*k2[x][y]) for x in range(3) for y in range(3)))
            radius=full[0]+t*(full[1]-full[0]);side_sign=1 if side==0 else -1
            ball.append([world[x]+side_sign*radius*n[x] for x in range(3)]);samples+=1
        centers=max(centers,math.dist(*ball))
    assert position<1e-7 and normal<1e-5 and tensor<1e-4 and centers<1e-7
    report=dict(proposalSamplesPassed=True,savedBodyAdmitted=False,nativeWholeSeamG2Proven=False,samples=samples,maxPositionErrorMm=position,maxNormalChord=normal,maxShapeTensorDifference=tensor,maxBallCenterGapMm=centers,sha256=hashlib.sha256(path.read_bytes()).hexdigest(),scope='sampled proposal diagnostics only; no final B-rep or continuous G2 certificate')
    (directory/'independent-check.json').write_text(json.dumps(report,indent=2)+'\n');reports.append(report)
assert len(reports)==2
(root/'independent-check.json').write_text(json.dumps(reports,indent=2)+'\n');print(json.dumps(reports,indent=2))
