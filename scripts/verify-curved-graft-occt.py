#!/usr/bin/env python3
"""Independent STEP validity, topology and bounds for the curved graft control.

Volume is measured independently and checked against the original analytic
source-volume upper bound; this is not an exact fillet-volume design oracle.
"""
import hashlib
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
from OCP.TopAbs import TopAbs_FACE, TopAbs_SOLID, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool


def count_unique(shape, kind):
    explorer, values = TopExp_Explorer(shape, kind), []
    while explorer.More():
        value = explorer.Current()
        if not any(value.IsSame(other) for other in values):
            values.append(value)
        explorer.Next()
    return len(values)


def main(directory):
    manifest = json.loads((directory / 'manifest.json').read_text())
    step = directory / manifest['file']
    if hashlib.sha256(step.read_bytes()).hexdigest() != manifest['stepSha256']:
        raise ValueError('STEP hash differs from native manifest')
    if hashlib.sha256((directory / 'model.json').read_bytes()).hexdigest() != manifest['modelSha256']:
        raise ValueError('Model hash differs from native manifest')
    reader = STEPControl_Reader()
    if reader.ReadFile(str(step)) != IFSelect_RetDone or reader.TransferRoots() == 0:
        raise ValueError('Independent STEP import failed')
    shape = reader.OneShape()
    valid = not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    bounds = [[p.X(), p.Y(), p.Z()] for p in [box.CornerMin(), box.CornerMax()]]
    expected = manifest['expected']
    error = max(abs(a-b) for row, wanted in zip(bounds, expected['boundsMm']) for a, b in zip(row, wanted))
    topology = {name: count_unique(shape, kind) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    properties = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, properties)
    volume = properties.Mass()
    tolerance = 1e-6  # Independent comparison tolerance; no native gate changes.
    explorer, points = TopExp_Explorer(shape, TopAbs_VERTEX), []
    while explorer.More():
        p = BRep_Tool.Pnt_s(TopoDS.Vertex(explorer.Current()))
        value = [p.X(), p.Y(), p.Z()]
        if not any(max(abs(a-b) for a, b in zip(value, other)) < 1e-10 for other in points):
            points.append(value)
        explorer.Next()
    radii = []
    for endpoint in manifest['radiusEndpoints']:
        near = [p for p in points if abs(p[1]-endpoint['yMm']) <= tolerance]
        floor = [p for p in near if abs(p[2]) <= tolerance and tolerance < p[0] < 10.-tolerance]
        cylinder = [p for p in near if tolerance < p[2] < 3.-tolerance
                    and abs((p[0]**2+(p[2]-3.)**2)**0.5-3.) <= tolerance]
        if len(floor) != 1 or len(cylinder) != 1:
            raise ValueError('Independent contact vertex identification failed')
        # Plane z=0 and cylinder radius 3, axis z=3: tangency of one contact
        # ball gives R=x_floor²/12=3*z_cylinder/(6-z_cylinder).
        first = floor[0][0]**2/12.
        second = 3.*cylinder[0][2]/(6.-cylinder[0][2])
        target = endpoint['expectedBallRadiusMm']
        radii.append(dict(yMm=endpoint['yMm'], authoredBallRadiusMm=target,
                          inferredFromFloorMm=first, inferredFromCylinderMm=second,
                          maximumErrorMm=max(abs(first-target), abs(second-target), abs(first-second))))
    passed = (valid and topology == {k: expected[k] for k in topology} and error <= tolerance
              and 0 < volume < manifest['sourceAnalyticVolumeUpperMm3']
              and all(r['maximumErrorMm'] <= tolerance for r in radii)
              and manifest['nativeClosureWithinTolerance'] and manifest['trimRegionsValid'])
    report = dict(schema='curved-variable-graft-occt/1', stepSha256=manifest['stepSha256'],
                  modelSha256=manifest['modelSha256'], valid=valid, topology=topology,
                  boundsMm=bounds, boundsErrorMm=error, measuredVolumeMm3=volume,
                  comparisonToleranceMm=tolerance, exactFilletVolumeClaim=False,
                  contactBallRadiusEndpoints=radii, wholeRadiusClaim=False,
                  wholeWallClaim=False, wholeBodyG2Claim=False, passed=passed)
    (directory / 'occt.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report, indent=2))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]).resolve()))
