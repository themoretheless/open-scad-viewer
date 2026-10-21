#!/usr/bin/env python3
"""Independent STEP and analytic contour check after automatic SS partition."""
import hashlib
import importlib.util
import json
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.Bnd import Bnd_Box
from OCP.BRepBndLib import BRepBndLib
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.BRepClass3d import BRepClass3d_SolidClassifier
from OCP.gp import gp_Pnt
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID, TopAbs_EDGE, TopAbs_VERTEX, TopAbs_IN, TopAbs_OUT


def main(directory):
    receipt = json.loads((directory/'manifest.json').read_text())
    for filename, key in [('partition.step', 'stepSha256'), ('model.json', 'modelSha256')]:
        assert hashlib.sha256((directory/filename).read_bytes()).hexdigest() == receipt[key]
    assert receipt['nativeClosed'] and receipt['nativeTrimValid']
    assert receipt['automaticContourClaim'] and receipt['codecRoundtrip'] and receipt['originalIdentitiesPreserved']
    assert receipt['cells'] <= 100000 and receipt['exactWork'] <= 1000000
    spec = importlib.util.spec_from_file_location('cap_check', Path(__file__).with_name('verify-curved-cap-occt.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(directory/'partition.step')) == IFSelect_RetDone and reader.TransferRoots()
    shape = reader.OneShape()
    assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    topology = {name: module.count(shape, kind) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    assert topology == dict(solids=1, faces=receipt['faces'], edges=receipt['edges'], vertices=receipt['vertices'])
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    bounds = [[p.X(), p.Y(), p.Z()] for p in [box.CornerMin(), box.CornerMax()]]
    bounds_error = max(abs(a-b) for row, target in zip(bounds, receipt['boundsMm']) for a, b in zip(row, target))
    assert bounds_error <= 2e-7
    props = GProp_GProps()
    integration_error = BRepGProp.VolumeProperties_s(shape, props, 1e-10)
    # Face partition preserves the original 2 x 2 x 1 cuboid.
    expected = 2*2*1
    assert receipt['expectedVolumeMm3'] == expected
    assert abs(props.Mass()-expected) <= 1e-7
    probes = [('original_material', (0., 0., .5), TopAbs_IN),
              ('below_bottom', (0., 0., -.1), TopAbs_OUT),
              ('outside_x', (1.1, 0., .5), TopAbs_OUT)]
    for _, point, wanted in probes:
        assert BRepClass3d_SolidClassifier(shape, gp_Pnt(*point), 1e-7).State() == wanted
    from OCP.TopExp import TopExp_Explorer
    from OCP.TopoDS import TopoDS
    from OCP.BRepAdaptor import BRepAdaptor_Curve
    from OCP.GeomAbs import GeomAbs_BSplineCurve
    explorer = TopExp_Explorer(shape, TopAbs_EDGE)
    found = []
    maximum_residual = 0.
    while explorer.More():
        edge = TopoDS.Edge_s(explorer.Current())
        if not any(edge.IsSame(old) for old in found):
            curve = BRepAdaptor_Curve(edge)
            if curve.GetType() == GeomAbs_BSplineCurve and curve.Degree() == 3:
                found.append(edge)
                lo, hi = curve.FirstParameter(), curve.LastParameter()
                for i in range(129):
                    point = curve.Value(lo+(hi-lo)*i/128)
                    residual = abs(point.Z()) + abs(point.X()**2+point.Y()**2-.5)
                    maximum_residual = max(maximum_residual, residual)
        explorer.Next()
    assert len(found) == len(receipt['contourEdges'])
    assert maximum_residual <= 1e-7
    report = dict(passed=True, topology=topology, volumeMm3=props.Mass(), expectedVolumeMm3=expected,
                  integrationRelativeError=integration_error, maximumBoundsErrorMm=bounds_error, boundsMm=bounds, probesPassed=[p[0] for p in probes],
                  contourEdges=len(found), maximumContourResidual=maximum_residual, curvedCapClaim=False, filletClaim=False)
    (directory/'independent.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main(Path(sys.argv[1]))
