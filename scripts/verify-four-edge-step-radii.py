#!/usr/bin/env python3
"""Independent numerical OCCT checks of native four-arm radius probes."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopExp import TopExp
from OCP.TopTools import TopTools_IndexedMapOfShape
from OCP.TopAbs import TopAbs_FACE
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepTools import BRepTools
from OCP.GeomLProp import GeomLProp_SLProps


def verify(root):
    path=root/'candidate.step'
    native=json.loads((root/'material-and-radius.json').read_text())
    reader=STEPControl_Reader()
    if reader.ReadFile(str(path))!=IFSelect_RetDone or reader.TransferRoots()!=1:
        raise ValueError('Expected one STEP root')
    faces=TopTools_IndexedMapOfShape()
    TopExp.MapShapes_s(reader.OneShape(),TopAbs_FACE,faces)
    arms=[]
    for i in range(1,faces.Extent()+1):
        face=TopoDS.Face_s(faces.FindKey(i))
        bounds=list(BRepTools.UVBounds_s(face))
        if all(abs(a-b)<=1e-9 for a,b in zip(bounds,[-3.,3.,0.,1.])):
            arms.append((i,BRep_Tool.Surface_s(face)))
    if len(arms)!=4:
        raise ValueError('Expected four complete arm charts')
    rows=[]
    for probe in native['independentNativeRadiusProbes']:
        u,v=probe['uv']
        matches=[]
        for i,s in arms:
            p=s.Value(u,v)
            point=[p.X(),p.Y(),p.Z()]
            if math.dist(point,probe['point'])<=1e-7:
                props=GeomLProp_SLProps(s,u,v,2,1e-12)
                if not props.IsCurvatureDefined():
                    raise ValueError('Undefined independent STEP curvature')
                curvatures=[props.MinCurvature(),props.MaxCurvature()]
                radius=1/max(abs(k) for k in curvatures)
                bounds=probe['actualRadiusInterval']
                matches.append(dict(stepFace=i,point=point,radiusMm=radius,
                    principalCurvaturesPerMm=curvatures,
                    withinNativeInterval=bounds[0]<=radius<=bounds[1],
                    requestedErrorMm=abs(radius-probe['requestedRadius'])))
        if len(matches)!=1:
            raise ValueError('Expected one geometry-matched arm per probe')
        row=dict(arm=probe['arm'],uv=probe['uv'],requestedRadiusMm=probe['requestedRadius'],**matches[0])
        row['passed']=row['withinNativeInterval'] and row['requestedErrorMm']<=1e-7
        rows.append(row)
    report=dict(occtVersion=OCP.__version__,stepSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        passed=len(rows)==12 and all(r['passed'] for r in rows),probes=rows,
        scope='12 numerical STEP principal-radius probes, independent of the native constructor formula; whole-face curvature and full G2 remain native interval gates.')
    (root/'step-radii.occt.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report,indent=2))
    return 0 if report['passed'] else 1


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('root',type=Path)
    raise SystemExit(verify(parser.parse_args().root))
