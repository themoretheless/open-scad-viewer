"""Independent STEP face association and selected opposing-face distances."""
import hashlib
import json
from pathlib import Path
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_FACE, TopAbs_IN
from OCP.TopoDS import TopoDS
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeVertex
from OCP.BRepExtrema import BRepExtrema_DistShapeShape
from OCP.BRepClass import BRepClass_FaceClassifier
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.gp import gp_Pnt


def measure(a, b):
    result = BRepExtrema_DistShapeShape(a, b)
    assert result.IsDone() and result.NbSolution() > 0
    return result


def main(directory):
    manifest = json.loads((directory/'manifest.json').read_text())
    rows = []
    for specimen in manifest['specimens']:
        path = directory/(specimen['name']+'.step')
        reader = STEPControl_Reader()
        assert reader.ReadFile(str(path)) == IFSelect_RetDone
        assert reader.TransferRoots() == 1
        shape = reader.OneShape()
        assert BRepCheck_Analyzer(shape).IsValid(), specimen['name']
        faces = []
        explorer = TopExp_Explorer(shape, TopAbs_FACE)
        while explorer.More():
            face = (getattr(TopoDS, 'Face_s', None) or TopoDS.Face)(explorer.Current())
            box = Bnd_Box()
            BRepBndLib.AddOptimal_s(face, box, False, False)
            lo, hi = box.CornerMin(), box.CornerMax()
            faces.append((face, (lo.X(), lo.Y(), lo.Z(), hi.X(), hi.Y(), hi.Z())))
            explorer.Next()
        assert len(faces) == 27
        mapped = {}
        for anchor in specimen['anchors']:
            point = anchor['point']
            vertex = BRepBuilderAPI_MakeVertex(gp_Pnt(*point)).Vertex()
            matches = []
            for index, (face, bounds) in enumerate(faces):
                if any(point[k] < bounds[k]-1e-7 or point[k] > bounds[k+3]+1e-7 for k in range(3)):
                    continue
                if BRepClass_FaceClassifier(face, gp_Pnt(*point), 1e-8).State() != TopAbs_IN:
                    continue
                if measure(vertex, face).Value() <= 1e-7:
                    matches.append(index)
            assert len(matches) == 1, (specimen['name'], anchor, matches)
            mapped[anchor['face']] = matches[0]
        assert len(set(mapped.values())) == len(mapped)
        for case in specimen['cases']:
            a, b = [faces[mapped[index]][0] for index in case['faces']]
            distance = measure(a, b)
            actual = distance.Value()
            lo, hi = case['result']['distanceIntervalMm']
            expected = case['expectedMm']
            # OCCT is an independent numeric check, not an interval proof.
            error = abs(actual-expected)
            passed = case['passed'] and lo-1e-7 <= actual <= hi+1e-7 and error <= 1e-7
            p, q = distance.PointOnShape1(1), distance.PointOnShape2(1)
            rows.append(dict(specimen=specimen['name'], importedShapeValid=True, parameters=specimen['parameters'],
                             direction=specimen['direction'], name=case['name'],
                             stepSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                             originalFaces=case['faces'], importedFaces=[mapped[i] for i in case['faces']],
                             expectedMm=expected, intervalMm=[lo, hi], independentDistanceMm=actual,
                             errorMm=error, pointsMm=[[p.X(), p.Y(), p.Z()], [q.X(), q.Y(), q.Z()]],
                             passed=passed))
    report = dict(schema='cad-opposing-clearance-occt/1', passed=all(r['passed'] for r in rows),
                  scope='Enumerated selected opposing faces; not global thickness or G1.',
                  faceAssociation='Unique interior point classified inside face and within 1e-7 mm of its surface.',
                  cases=rows)
    (directory/'occt-report.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(dict(passed=report['passed'], cases=len(rows), maxErrorMm=max(r['errorMm'] for r in rows))))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1])))
