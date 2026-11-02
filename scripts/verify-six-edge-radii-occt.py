#!/usr/bin/env python3
"""Independently measure actual exported arm principal radii at native probes."""
import argparse,json,math,hashlib
from pathlib import Path
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopTools import TopTools_IndexedMapOfShape
from OCP.TopExp import TopExp
from OCP.TopAbs import TopAbs_FACE,TopAbs_IN
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepTools import BRepTools
from OCP.BRepClass import BRepClass_FaceClassifier
from OCP.GeomLProp import GeomLProp_SLProps
from OCP.gp import gp_Pnt2d

def main():
    p=argparse.ArgumentParser();p.add_argument('directory',type=Path);d=p.parse_args().directory
    native=json.loads((d/'report.json').read_text());probes=json.loads((d/'radii.json').read_text());path=d/'candidate.step'
    reader=STEPControl_Reader();assert reader.ReadFile(str(path))==IFSelect_RetDone;assert reader.TransferRoots()>0
    faces=TopTools_IndexedMapOfShape();TopExp.MapShapes_s(reader.OneShape(),TopAbs_FACE,faces)
    rows=[]
    for i,probe in enumerate(probes):
        u,v=probe['uv'];hits=[]
        for f in range(1,faces.Extent()+1):
            face=TopoDS.Face_s(faces.FindKey(f));umin,umax,vmin,vmax=BRepTools.UVBounds_s(face)
            if not(umin-1e-12<=u<=umax+1e-12 and vmin-1e-12<=v<=vmax+1e-12):continue
            surf=BRep_Tool.Surface_s(face);point=surf.Value(u,v);xyz=[point.X(),point.Y(),point.Z()]
            error=math.dist(xyz,probe['point'])
            if error>1e-7 or BRepClass_FaceClassifier(face,gp_Pnt2d(u,v),1e-9).State()!=TopAbs_IN:continue
            props=GeomLProp_SLProps(surf,u,v,2,1e-12)
            if not props.IsCurvatureDefined():continue
            ks=[props.MinCurvature(),props.MaxCurvature()];radius=1/max(map(abs,ks));lo,hi=probe['principalRadiusIntervalMm']
            requested=native.get('requestedMidRadii');target=None if requested is None else requested[i//2]
            hits.append(dict(face=f,pointErrorMm=error,radiusMm=radius,nativeIntervalMm=[lo,hi],
                nativeIntervalMatch=lo-1e-9<=radius<=hi+1e-9,requestedRadiusMm=target,requestedMatch=target is None or abs(radius-target)<=1e-7))
        rows.append(dict(probe=i,passed=len(hits)==1 and hits[0]['nativeIntervalMatch'] and hits[0]['requestedMatch'],matches=hits))
    result=dict(passed=all(r['passed'] for r in rows) and len(rows)==6,occtVersion=OCP.__version__,
        inputSha256=hashlib.sha256(path.read_bytes()).hexdigest(),probes=rows,
        scope='Six independent STEP principal-radius points; complete native curvature/G2 proof is separate. Numerical interval comparison allowance 1e-9 mm.')
    (d/'candidate.radii.occt.json').write_text(json.dumps(result,indent=2)+'\n');print(json.dumps(result,indent=2))
    if not result['passed']:raise SystemExit(1)
if __name__=='__main__':main()
