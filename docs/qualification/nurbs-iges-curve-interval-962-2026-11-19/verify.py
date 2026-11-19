"""Compare the source interval against the Rust-trimmed IGES spline in OCCT."""
import sys
from pathlib import Path
from OCP.IFSelect import IFSelect_RetDone
from OCP.IGESControl import IGESControl_Reader
from OCP.IGESToBRep import IGESToBRep_BasicCurve

curves = []
for filename in ("source.igs", "trimmed.igs"):
    reader = IGESControl_Reader()
    assert reader.ReadFile(str(Path(sys.argv[1]) / filename)) == IFSelect_RetDone
    model = reader.IGESModel()
    spline = model.Entity(model.NbEntities())
    assert spline.TypeNumber() == 126
    assert (spline.UMin(), spline.UMax()) == (0.25, 1.5)
    curves.append(IGESToBRep_BasicCurve().TransferBSplineCurve(spline))
errors = [curves[0].Value(t).Distance(curves[1].Value(t))
          for t in [0.25 + 1.25 * i / 128 for i in range(129)]]
assert max(errors) < 1e-12
print("maximum distance:", max(errors))
