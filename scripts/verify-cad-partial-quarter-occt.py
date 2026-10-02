"""Check native STEP against an independently integrated removed-material volume."""
import hashlib
import json
import math
from pathlib import Path
import sys

from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box


def expected_volume(intervals, parameters=(20., 5., 6., 1.25)):
    outer, inner, height, radius = parameters
    angle = math.pi / 8

    def moment(r):
        return (outer - r) * (1 - math.pi / 4) * r*r + r**3 / 6

    def integrand(u):
        # Rational quadratic angular parameter, independent of exported poles.
        x = (1-u)**2 + 2*u*(1-u)*math.cos(angle/2) + u*u*math.cos(angle)
        y = 2*u*(1-u)*math.sin(angle/2) + u*u*math.sin(angle)
        dx = -2*(1-u) + 2*(1-2*u)*math.cos(angle/2) + 2*u*math.cos(angle)
        dy = 2*(1-2*u)*math.sin(angle/2) + 2*u*math.sin(angle)
        derivative = (x*dy-y*dx)/(x*x+y*y)
        smooth = 3*u*u-2*u**3
        return (moment(radius*smooth)+moment(radius*(1-smooth))) * derivative

    total = integrand(0.) + integrand(1.)
    total += sum((4 if i % 2 else 2)*integrand(i/intervals) for i in range(1, intervals))
    removed = total/(3*intervals) + moment(radius)*math.pi/4
    return math.pi*(outer*outer-inner*inner)*height - removed


def main(directory):
    manifest_path = directory/'manifest.json'
    cases = json.loads(manifest_path.read_text())['cases'] if manifest_path.exists() else [
        dict(name=name, parameters=[20., 5., 6., 1.25]) for name in ['negative', 'positive']]
    convergence = 0.
    rows = []
    for case in cases:
        name = case['name']
        expected = expected_volume(8192, case['parameters'])
        convergence = max(convergence, abs(expected-expected_volume(4096, case['parameters'])))
        path = directory / (name+'.step')
        reader = STEPControl_Reader()
        assert reader.ReadFile(str(path)) == IFSelect_RetDone
        assert reader.TransferRoots() == 1
        shape = reader.OneShape()
        explorer = TopExp_Explorer(shape, TopAbs_SOLID)
        solids = 0
        while explorer.More():
            solids += 1
            explorer.Next()
        props = GProp_GProps()
        integration = BRepGProp.VolumeProperties_s(shape, props, Eps=1e-12)
        error = abs(props.Mass()-expected)
        valid = BRepCheck_Analyzer(shape).IsValid()
        box = Bnd_Box()
        BRepBndLib.AddOptimal_s(shape, box, False, False)
        actual_bounds = [[p.X(), p.Y(), p.Z()] for p in [box.CornerMin(), box.CornerMax()]]
        outer, _, height, _ = case['parameters']
        expected_bounds = [[-outer, -outer, 0.], [outer, outer, height]]
        bounds_error = max(abs(a-b) for actual, target in zip(actual_bounds, expected_bounds)
                           for a, b in zip(actual, target))
        rows.append(dict(name=name, sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                         parameters=case['parameters'], valid=valid, solids=solids, volumeMm3=props.Mass(),
                         expectedVolumeMm3=expected, volumeErrorMm3=error,
                         boundsMm=actual_bounds, boundsErrorMm=bounds_error,
                         integrationRelativeError=integration,
                         passed=valid and solids == 1 and error <= 1e-6 and bounds_error <= 1e-6))
    report = dict(schema='cad-partial-quarter-occt/1',
                  scope='Enumerated quarter-rim specimens; numerical volume and OCCT topology validity. General self-intersections and whole-domain tangency remain unqualified.',
                  oracleConvergenceMm3=convergence,
                  passed=convergence <= 1e-8 and all(row['passed'] for row in rows), parts=rows)
    (directory/'occt-report.json').write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]).resolve()))
