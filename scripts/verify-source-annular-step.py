"""Independent OCCT qualification of the specified original annular STEP.
Checks original pole topology, solid validity, extents and volume against
an independent coefficient-quadrature result. No arbitrary-fillet claim.
"""
import hashlib
import json
from pathlib import Path
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box


def unique(shape, kind):
    result = []
    it = TopExp_Explorer(shape, kind)
    while it.More():
        item = it.Current()
        if not any(item.IsSame(old) for old in result):
            result.append(item)
        it.Next()
    return result


def cast(name, shape):
    fn = getattr(TopoDS, name+'_s', None) or getattr(TopoDS, name)
    return fn(shape)


def main(step, oracle_path, output):
    oracle = json.loads(oracle_path.read_text())
    assert oracle['schema'] == 'cad-original-annular-flux-quadrature/1' and oracle['passed']
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(step)) == IFSelect_RetDone
    assert reader.TransferRoots() == 1
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    solids, faces = unique(shape, TopAbs_SOLID), unique(shape, TopAbs_FACE)
    edges, vertices = unique(shape, TopAbs_EDGE), unique(shape, TopAbs_VERTEX)
    degenerate = [cast('Edge', e) for e in edges if BRep_Tool.Degenerated_s(cast('Edge', e))]
    pole_points = []
    for edge in degenerate:
        ends = unique(edge, TopAbs_VERTEX)
        assert len(ends) == 1
        p = BRep_Tool.Pnt_s(cast('Vertex', ends[0]))
        pole_points.append([p.X(), p.Y(), p.Z()])
    expected_poles = [[20., 0., 6.], [0., 20., 6.]]
    distance = lambda p, q: max(abs(a-b) for a, b in zip(p, q))
    pole_error = max(max(min(distance(p, q) for q in expected_poles) for p in pole_points),
                     max(min(distance(p, q) for p in pole_points) for q in expected_poles)) if pole_points else float('inf')
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-10, True)
    volume = props.Mass()
    expected_volume = oracle['signedVolumeMm3'][-1]
    volume_error = abs(volume-expected_volume)
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    lo, hi = box.CornerMin(), box.CornerMax()
    bounds = [lo.X(), lo.Y(), lo.Z(), hi.X(), hi.Y(), hi.Z()]
    expected_bounds = [-20., -20., 0., 20., 20., 6.]
    bounds_error = max(abs(a-b) for a, b in zip(bounds, expected_bounds))
    passed = (valid and len(solids) == 1 and len(faces) == 27 and len(edges) == 55
              and len(vertices) == 26 and len(degenerate) == 2 and pole_error < 1e-7
              and volume_error < 1e-5 and bounds_error < 1e-6)
    report = dict(schema='cad-original-annular-step-occt/1',
                  stepSha256=hashlib.sha256(step.read_bytes()).hexdigest(),
                  oracleSha256=hashlib.sha256(oracle_path.read_bytes()).hexdigest(),
                  occtValid=valid, solids=len(solids), faces=len(faces), edges=len(edges), vertices=len(vertices),
                  degenerateEdges=len(degenerate), polePointsMm=pole_points, poleErrorMm=pole_error,
                  volumeMm3=volume, expectedVolumeMm3=expected_volume, volumeErrorMm3=volume_error,
                  boundsMm=bounds, expectedBoundsMm=expected_bounds, maxBoundsErrorMm=bounds_error,
                  scope='One native original annular source STEP: independent OCCT validity, unique topology, poles, extents and volume; no edit, Undo/Redo or arbitrary fillet proof.', passed=passed)
    output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])))
