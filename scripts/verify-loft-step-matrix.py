"""Verify generated STEP matrix against source-surface samples using OpenCascade."""
import json
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.gp import gp_Pnt, gp_Vec

root = Path(sys.argv[1])
reports = []
for samples_path in sorted(root.glob('*.samples.json')):
    name = samples_path.name.removesuffix('.samples.json')
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(root / (name+'.step'))) == IFSelect_RetDone, name
    assert reader.TransferRoots() == 1, name
    shape = reader.OneShape()
    assert BRepCheck_Analyzer(shape).IsValid(), name
    faces = TopExp_Explorer(shape, TopAbs_FACE)
    assert faces.More(), name
    surface = BRepAdaptor_Surface(TopoDS.Face(faces.Current()))
    faces.Next()
    assert not faces.More(), name
    assert not TopExp_Explorer(shape, TopAbs_SOLID).More(), name
    samples = json.loads(samples_path.read_text())
    error = max(abs(a-b) for sample in samples for a,b in zip(surface.Value(sample['u'],sample['v']).Coord(),sample['point']))
    assert error < 1e-9, (name,error)
    derivative_error = 0.0
    for sample in samples:
        if 'dv' in sample:
            p,du,dv=gp_Pnt(),gp_Vec(),gp_Vec()
            surface.D1(sample['u'],sample['v'],p,du,dv)
            derivative_error=max(derivative_error,*(abs(a-b) for a,b in zip(dv.Coord(),sample['dv'])))
    assert derivative_error < 1e-9, (name,derivative_error)
    reports.append(dict(case=name,valid=True,samples=len(samples),max_position_error=error,v_derivative_samples=sum('dv' in sample for sample in samples),max_v_derivative_error=derivative_error if any('dv' in sample for sample in samples) else None))
assert len(reports) >= 4
result = dict(parser='OpenCascade/cadquery-ocp',cases=reports,scope='sampled STEP/source preservation; not a continuous error certificate')
(root/'opencascade-matrix.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
