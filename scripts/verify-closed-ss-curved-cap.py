#!/usr/bin/env python3
"""Independent STEP check of a curved cap bounded by an original-support SS contour."""
import math
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
    for filename, key in [('curved-cap.step', 'stepSha256'), ('model.json', 'modelSha256')]:
        assert hashlib.sha256((directory/filename).read_bytes()).hexdigest() == receipt[key]
    assert receipt['nativeClosed'] and receipt['nativeTrimValid']
    assert receipt['closedContourConstructedFromSupports'] and receipt['capInjective']
    assert receipt['cells'] <= 100000 and receipt['exactWork'] <= 1000000
    spec = importlib.util.spec_from_file_location('cap_check', Path(__file__).with_name('verify-curved-cap-occt.py'))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(directory/'curved-cap.step')) == IFSelect_RetDone and reader.TransferRoots()
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
    # Integral of (0.5-r*r) over the disk r <= sqrt(0.5), plus the original cube.
    expected = 4 + math.pi/8
    assert receipt['expectedVolumeMm3'] == expected
    assert abs(props.Mass()-expected) <= 1e-7
    probes = [('added_center', (0., 0., -.25), TopAbs_IN),
              ('below_cap', (0., 0., -.6), TopAbs_OUT),
              ('outside_disk', (.9, .9, -.1), TopAbs_OUT),
              ('original_material', (.9, .9, .5), TopAbs_IN),
              ('inside_bowl', (.4, 0., -.1), TopAbs_IN),
              ('outside_bowl', (.7, 0., -.1), TopAbs_OUT)]
    for _, point, wanted in probes:
        assert BRepClass3d_SolidClassifier(shape, gp_Pnt(*point), 1e-7).State() == wanted
    report = dict(passed=True, topology=topology, volumeMm3=props.Mass(), expectedVolumeMm3=expected,
                  integrationRelativeError=integration_error, maximumBoundsErrorMm=bounds_error, boundsMm=bounds, probesPassed=[p[0] for p in probes],
                  completeSurfaceIntersectionClaim=False, globalEmbeddingClaim=False, filletClaim=False)
    (directory/'independent.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    main(Path(sys.argv[1]))
