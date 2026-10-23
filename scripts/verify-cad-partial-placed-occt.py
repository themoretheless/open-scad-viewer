"""Source-bound partial-rim STEP checks against independently placed base solids."""
import hashlib
import json
from pathlib import Path
import runpy
import sys
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.gp import gp_Trsf
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID


def main(directory):
    scripts = Path(__file__).resolve().parent
    helpers = runpy.run_path(str(scripts/'verify-cad-rigid-step-occt.py'))
    expected_volume = runpy.run_path(str(scripts/'verify-cad-partial-quarter-occt.py'))['expected_volume'](8192)
    read, bounds = helpers['read'], helpers['bounds']
    rows = []
    manifest = json.loads((directory/'manifest.json').read_text())
    for part in manifest['parts']:
        path = directory/part['file']
        shape = read(path)
        reference = read(directory/f"placement-0-edge-{part['edge']}.step")
        if part['placement']:
            transform = gp_Trsf()
            transform.SetValues(*[v for row in part['matrix'][:3] for v in row])
            reference = BRepBuilderAPI_Transform(reference, transform, True).Shape()
        error_bounds = max(abs(a-b) for actual,target in zip(bounds(shape),bounds(reference))
                           for a,b in zip(actual,target))
        props = GProp_GProps()
        BRepGProp.VolumeProperties_s(shape, props, Eps=1e-12)
        error_volume = abs(props.Mass()-expected_volume)
        explorer = TopExp_Explorer(shape, TopAbs_SOLID)
        solids = 0
        while explorer.More():
            solids += 1
            explorer.Next()
        valid = BRepCheck_Analyzer(shape).IsValid()
        rows.append(dict(file=part['file'], sha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                         valid=valid, solids=solids, boundsErrorMm=error_bounds,
                         volumeErrorMm3=error_volume,
                         passed=valid and solids==1 and error_bounds<=1e-6 and error_volume<=1e-6))
    report = dict(schema='cad-partial-annular-placed-occt/1',passed=all(r['passed'] for r in rows),
                  scope='Enumerated source arcs and rigid placements; numeric volume, bounds and OCCT validity, not a whole-boundary intersection proof.',parts=rows)
    (directory/'occt-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(passed=report['passed'],cases=len(rows),
                         maxBoundsErrorMm=max(r['boundsErrorMm'] for r in rows),
                         maxVolumeErrorMm3=max(r['volumeErrorMm3'] for r in rows))))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]).resolve()))
