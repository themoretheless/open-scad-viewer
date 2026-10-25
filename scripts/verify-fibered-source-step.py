"""Independent OCCT checks for the four specified Rust original-fibered disk STEP fixtures.
Rust supplies original-source flux quadrature and points. These numerical
references are supplementary evidence, not analytic continuous-bound proofs.
"""
import hashlib
import json
import math
from pathlib import Path
import sys
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.gp import gp_Pnt


def unique(shape, kind):
    out = []
    it = TopExp_Explorer(shape, kind)
    while it.More():
        item = it.Current()
        if not any(item.IsSame(x) for x in out):
            out.append(item)
        it.Next()
    return out


def static(owner, name):
    return getattr(owner, name + '_s', None) or getattr(owner, name)


def check(step):
    reference_path = step.parent/'reference.json'
    audit_path = step.parent/'audited.json'
    reference = json.loads(reference_path.read_text())
    audit = json.loads(audit_path.read_text())
    assert reference['schema'] == 'original-fibered-disk-numerical-reference/1'
    assert reference['passed'] and audit['certified'] and audit['sourceBoundaryExact']
    assert audit['fullBoundaryG2Certified'] and audit['idealSourceBoundaryG2Certified']
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(step)) == IFSelect_RetDone
    assert reader.TransferRoots() == 1
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    kinds = [TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX]
    solids, faces, edges, vertices = [unique(shape, k) for k in kinds]
    degenerate = sum(bool(static(BRep_Tool, 'Degenerated')(static(TopoDS, 'Edge')(e))) for e in edges)
    adjacency = []
    for edge in edges:
        adjacency.append(sum(any(edge.IsSame(e) for e in unique(face, TopAbs_EDGE)) for face in faces))
    props = GProp_GProps()
    static(BRepGProp, 'VolumeProperties')(shape, props, 1e-10, True)
    volume = props.Mass()
    expected = reference['volumeMm3'][-1]
    error = abs(volume - expected)
    distances = []
    for p in reference['samplesMm']:
        vertex = BRepBuilderAPI_MakeVertex(gp_Pnt(*p)).Vertex()
        query = BRepExtrema_DistShapeShape(vertex, shape)
        query.Perform()
        assert query.IsDone()
        distances.append(query.Value())
    max_distance = max(distances)
    built = json.loads((step.parent/'built.json').read_text())['model']
    passed = (valid and len(solids) == 1 and len(faces) == len(built['faces']) and len(edges) == len(built['edges'])
              and len(vertices) == len(built['vertices']) and degenerate == 0 and all(n == 2 for n in adjacency)
              and math.isfinite(volume) and volume > 0 and error < 1e-5 and max_distance < 1e-7)
    return dict(name=step.parent.name, passed=passed, stepSha256=hashlib.sha256(step.read_bytes()).hexdigest(),
                rustAuditSha256=hashlib.sha256(audit_path.read_bytes()).hexdigest(),
                rustReferenceSha256=hashlib.sha256(reference_path.read_bytes()).hexdigest(),
                occtValid=valid, solids=len(solids), faces=len(faces), edges=len(edges), vertices=len(vertices),
                degenerateEdges=degenerate, twoFaceEdges=sum(n == 2 for n in adjacency),
                volumeMm3=volume, expectedVolumeMm3=expected, volumeErrorMm3=error,
                inspectedSourcePoints=len(distances), maxSourcePointDistanceMm=max_distance)


def main(directory, output):
    cases = [check(p) for p in sorted(directory.glob('*/body.step'))]
    passed = len(cases) == 4 and all(c['passed'] for c in cases)
    result = dict(schema='cad-original-fibered-disk-occt/1', ocpVersion=OCP.__version__,
                  scope='Four specified original rational-affine disk fiber STEP fixtures: independent OCCT validity, topology, orientation, numerical volume and source-point checks; exact original/actual G2 remains a separate native proof.',
                  cases=cases, passed=passed)
    output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main(Path(sys.argv[1]), Path(sys.argv[2])))
