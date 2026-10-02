"""Independent rigid-placement STEP checks against OCCT-transformed base exports."""
import hashlib
import json
from pathlib import Path
import sys
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepBuilderAPI import BRepBuilderAPI_Transform
from OCP.BRepBndLib import BRepBndLib
from OCP.Bnd import Bnd_Box
from OCP.GProp import GProp_GProps
from OCP.BRepGProp import BRepGProp
from OCP.gp import gp_Trsf
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID

def read(path):
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(path)) == IFSelect_RetDone and reader.TransferRoots() > 0
    return reader.OneShape()

def bounds(shape):
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    return [[p.X(), p.Y(), p.Z()] for p in [box.CornerMin(), box.CornerMax()]]

def main(directory):
    manifest = json.loads((directory / 'manifest.json').read_text())
    entries = {p['file']: p for p in manifest['parts']}
    rows = []
    for part in manifest['parts']:
        path = directory / part['file']
        assert hashlib.sha256(path.read_bytes()).hexdigest() == part['sha256']
        shape = read(path)
        expected = part['expected']
        if 'referenceFile' in expected:
            reference_path = directory / expected['referenceFile']
            assert hashlib.sha256(reference_path.read_bytes()).hexdigest() == entries[expected['referenceFile']]['sha256']
            matrix = expected['rigidMatrix']
            transform = gp_Trsf()
            transform.SetValues(*[v for row in matrix[:3] for v in row])
            reference = BRepBuilderAPI_Transform(read(reference_path), transform, True).Shape()
            target_bounds = bounds(reference)
        else:
            target_bounds = expected['boundsMm']
        actual_bounds = bounds(shape)
        bounds_error = max(abs(a-b) for actual, target in zip(actual_bounds, target_bounds) for a,b in zip(actual,target))
        props = GProp_GProps()
        integration_error = BRepGProp.VolumeProperties_s(shape, props, Eps=1e-10)
        volume_error = abs(props.Mass()-expected['volumeMm3'])
        explorer = TopExp_Explorer(shape, TopAbs_SOLID)
        solids = 0
        while explorer.More():
            solids += 1
            explorer.Next()
        valid = BRepCheck_Analyzer(shape).IsValid()
        passed = valid and solids == 1 and bounds_error <= manifest['toleranceMm'] and volume_error <= max(1e-6, abs(expected['volumeMm3'])*manifest['relativeVolumeTolerance'])
        rows.append(dict(name=part['name'],valid=valid,solids=solids,boundsErrorMm=bounds_error,volumeErrorMm3=volume_error,integrationRelativeError=integration_error,passed=passed))
    report = dict(schema='cad-rigid-step-occt/1',passed=all(r['passed'] for r in rows),scope='Numerical validity, analytical volume and bounds against independent OCCT rigid transform; does not prove general geometry or surface continuity.',parts=rows)
    (directory/'occt-rigid-report.json').write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(dict(passed=report['passed'],cases=len(rows),maxBoundsErrorMm=max(r['boundsErrorMm'] for r in rows),maxVolumeErrorMm3=max(r['volumeErrorMm3'] for r in rows))))
    return 0 if report['passed'] else 1

if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]).resolve()))
