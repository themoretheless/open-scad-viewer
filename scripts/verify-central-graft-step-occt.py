#!/usr/bin/env python3
"""Independent STEP transfer/basic topology check; never a solid/embedding proof."""
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
from OCP.TopAbs import TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX, TopAbs_SOLID, TopAbs_SHELL
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps

def tile_witness(shape, report_path):
    from OCP.BRep import BRep_Tool
    from OCP.BRepTools import BRepTools
    from OCP.BRepClass import BRepClass_FaceClassifier
    from OCP.TopAbs import TopAbs_IN
    from OCP.TopoDS import TopoDS
    from OCP.gp import gp_Pnt2d
    from OCP.GeomLProp import GeomLProp_SLProps
    data = json.loads(Path(report_path).read_text())
    uv = data['witnessUV']
    rectangle = data['rectangle']
    expected_bounds = rectangle[0] + rectangle[1]
    faces = TopTools_IndexedMapOfShape()
    TopExp.MapShapes_s(shape, TopAbs_FACE, faces)
    matches = []
    for index in range(1, faces.Extent()+1):
        face = TopoDS.Face_s(faces.FindKey(index))
        bounds = list(BRepTools.UVBounds_s(face))
        if all(abs(a-b) <= 1e-9 for a,b in zip(bounds,expected_bounds)):
            surface = BRep_Tool.Surface_s(face)
            p = surface.Value(*uv)
            props = GeomLProp_SLProps(surface,*uv,2,1e-12)
            curvatures = [props.MinCurvature(),props.MaxCurvature()] if props.IsCurvatureDefined() else None
            point = [p.X(),p.Y(),p.Z()]
            inside = BRepClass_FaceClassifier(face,gp_Pnt2d(*uv),1e-9).State() == TopAbs_IN
            error = math.dist(point,data['after'])
            matches.append(dict(faceIndex=index,uvBounds=bounds,point=point,
                expectedPoint=data['after'],errorMm=error,parameterInside=inside,
                principalCurvaturesPerMm=curvatures,
                principalRadiiMm=[1/abs(k) if abs(k)>1e-15 else None for k in curvatures] if curvatures else None,
                interiorCurvatureQualified=False,
                inOriginalCubeAtWitness=all(-1e-7 <= x <= 10+1e-7 for x in point),
                passed=inside and error <= 1e-7,
                scope='One STEP surface point and face membership; not whole-face or body material containment'))
    if len(matches) != 1:
        return dict(passed=False,error='Expected exactly one tile with the authored UV bounds',matches=len(matches))
    return matches[0]


def verify(path, expected_counts=None, tile_report=None):
    expected_counts = expected_counts or dict(faces=13, edges=33, vertices=22, solids=1, shells=1)
    if any(v <= 0 for v in expected_counts.values()):
        raise ValueError("Expected topology counts must be positive")
    reader = STEPControl_Reader()
    readable = reader.ReadFile(str(path)) == IFSelect_RetDone
    roots = reader.TransferRoots() if readable else 0
    if not readable or roots == 0:
        report = dict(occtVersion=OCP.__version__, inputSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
            transferredRoots=roots, basicBrepValid=False,
            error='No transferred STEP roots' if readable else 'STEP read failed',
            globalEmbeddingQualified=False, selfIntersectionsQualified=False, materialQualified=False)
        path.with_suffix('.occt.json').write_text(json.dumps(report, indent=2)+'\n')
        print(json.dumps(report, indent=2))
        return 1
    shape = reader.OneShape()
    counts = {}
    for name, kind in [('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX),
                       ('solids', TopAbs_SOLID), ('shells', TopAbs_SHELL)]:
        entities = TopTools_IndexedMapOfShape()
        TopExp.MapShapes_s(shape, kind, entities)
        counts[name] = entities.Extent()
    properties = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, properties)
    valid = BRepCheck_Analyzer(shape, True).IsValid()
    witness = tile_witness(shape,tile_report) if tile_report else None
    report = dict(occtVersion=OCP.__version__, inputSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
        transferredRoots=roots, counts=counts, expectedCounts=expected_counts, topologyMatchesExpected=counts==expected_counts, basicBrepValid=valid,
        volumeEstimateMm3=properties.Mass(), tileWitness=witness, globalEmbeddingQualified=False,
        selfIntersectionsQualified=False, materialQualified=False,
        scope='Independent STEP representation/basic topology and numerical volume only; not a global solid certificate')
    path.with_suffix('.occt.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
    return 0 if valid and counts == expected_counts and properties.Mass() > 0 and (witness is None or witness["passed"]) else 1

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('path', type=Path)
    parser.add_argument('--faces',type=int,default=13)
    parser.add_argument('--edges',type=int,default=33)
    parser.add_argument('--vertices',type=int,default=22)
    parser.add_argument('--tile-report',type=Path)
    args=parser.parse_args()
    raise SystemExit(verify(args.path,dict(faces=args.faces,edges=args.edges,vertices=args.vertices,solids=1,shells=1),args.tile_report))
