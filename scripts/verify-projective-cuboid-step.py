"""Independent OCCT check of the projective 10 x 20 x 30 mm cuboid control.
This is bounded source-carrier exchange evidence, not general fillet acceptance.
"""
import hashlib
import json
import math
from pathlib import Path
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.TopExp import TopExp
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
try:
    from OCP.collections import IndexedMap_TopoDS_Shape_TopTools_ShapeMapHasher as ShapeMap
except ImportError:
    from OCP.TopTools import TopTools_IndexedMapOfShape as ShapeMap
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool


def inspect(step_path, output):
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(step_path)) == IFSelect_RetDone
    assert reader.TransferRoots() > 0
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    counts = {}
    vertices = []
    for name, kind in [('solids', TopAbs_SOLID), ('faces', TopAbs_FACE),
                       ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]:
        table = ShapeMap()
        TopExp.MapShapes_s(shape, kind, table)
        counts[name] = table.Extent()
        if name == 'vertices':
            for i in range(1, table.Extent() + 1):
                point = BRep_Tool.Pnt_s((getattr(TopoDS, 'Vertex_s', None) or TopoDS.Vertex)(table.FindKey(i)))
                vertices.append([point.X(), point.Y(), point.Z()])
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-10, True)
    volume = props.Mass()
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    lo, hi = box.CornerMin(), box.CornerMax()
    bounds = [lo.X(), lo.Y(), lo.Z(), hi.X(), hi.Y(), hi.Z()]
    bound_error = max(abs(a-b) for a,b in zip(bounds, [0.,0.,0.,10.,20.,30.]))
    expected_vertices = [[x,y,z] for x in [0.,10.] for y in [0.,20.] for z in [0.,30.]]
    vertex_error = max(min(math.dist(p,q) for q in vertices) for p in expected_vertices)
    passed = (valid and counts == dict(solids=1,faces=6,edges=12,vertices=8)
              and abs(volume-6000.) < 1e-7 and bound_error < 1e-6 and vertex_error < 1e-7)
    report = dict(schema='cad-projective-cuboid-step-occt/1',
                  scope='Native projective cuboid: independent topology, volume, bounds and all original corner positions',
                  stepSha256=hashlib.sha256(step_path.read_bytes()).hexdigest(),
                  occtValid=valid, counts=counts, volumeMm3=volume,
                  expectedVolumeMm3=6000., volumeErrorMm3=abs(volume-6000.),
                  boundsMm=bounds, maxBoundsErrorMm=bound_error,
                  verticesMm=vertices, maxExpectedVertexErrorMm=vertex_error, passed=passed)
    output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(inspect(Path(sys.argv[1]), Path(sys.argv[2])))
