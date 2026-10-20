"""Independent OCCT acceptance of the specified source-body STEP control.
The fixture is C0=(10,-7,5), C1=(13,-3,17), radii=(.5,1.25).
This checks one native export, not arbitrary fillets or full-wall qualification.
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
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID


def main(step_path, output, fixture=None):
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(step_path)) == IFSelect_RetDone
    assert reader.TransferRoots() > 0
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    explorer = TopExp_Explorer(shape, TopAbs_SOLID)
    solids = 0
    while explorer.More():
        solids += 1
        explorer.Next()
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-10, True)
    volume = props.Mass()
    fixture = fixture or dict(centers=[[10.,-7.,5.],[13.,-3.,17.]], radii=[.5,1.25])
    c0, c1 = fixture['centers']
    r0, r1 = fixture['radii']
    assert len(c0) == len(c1) == 3
    assert all(math.isfinite(x) for x in [*c0,*c1,r0,r1])
    length = math.dist(c0,c1)
    assert r0 > 0 and r1 > 0 and length > abs(r1-r0)
    slope = (r1-r0)/length
    h0, h1 = r0*(1-slope), r1*(1+slope)
    expected = math.pi*(length*(1-slope*slope)**2*(r0*r0+r0*r1+r1*r1)/3
                       + h0*h0*(r0-h0/3) + h1*h1*(r1-h1/3))
    box = Bnd_Box()
    BRepBndLib.AddOptimal_s(shape, box, False, False)
    lo, hi = box.CornerMin(), box.CornerMax()
    actual_bounds = [lo.X(),lo.Y(),lo.Z(),hi.X(),hi.Y(),hi.Z()]
    expected_bounds = [min(a-r0,b-r1) for a,b in zip(c0,c1)] + [max(a+r0,b+r1) for a,b in zip(c0,c1)]
    bounds_error = max(abs(a-b) for a,b in zip(actual_bounds,expected_bounds))
    volume_error = abs(volume-expected)
    passed = valid and solids == 1 and volume_error < 1e-6 and bounds_error < 1e-6
    report = dict(schema='cad-source-capped-body-step-occt/1',
                  stepSha256=hashlib.sha256(step_path.read_bytes()).hexdigest(),
                  scope='One specified native source-body STEP export; independent solid validity, signed volume and axis extents.',
                  fixture=fixture, occtValid=valid, solids=solids, volumeMm3=volume,
                  expectedVolumeMm3=expected, volumeErrorMm3=volume_error,
                  boundsMm=actual_bounds, expectedBoundsMm=expected_bounds,
                  maxBoundsErrorMm=bounds_error, passed=passed)
    output.write_text(json.dumps(report,indent=2)+'\n')
    print(json.dumps(report))
    return 0 if passed else 1


if __name__ == '__main__':
    sys.exit(main(Path(sys.argv[1]),Path(sys.argv[2]),json.loads(Path(sys.argv[3]).read_text()) if len(sys.argv)>3 else None))
