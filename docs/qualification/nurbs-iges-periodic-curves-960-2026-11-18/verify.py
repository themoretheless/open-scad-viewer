"""Inspect the two final type-126 entities with Open CASCADE."""
import sys
from OCP.IFSelect import IFSelect_RetDone
from OCP.IGESControl import IGESControl_Reader

reader = IGESControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
model = reader.IGESModel()
for idx, expected in [(model.NbEntities() - 1, (False, True, False)),
                      (model.NbEntities(), (True, False, True))]:
    curve = model.Entity(idx)
    assert curve.TypeNumber() == 126
    flags = (curve.IsClosed(), curve.IsPolynomial(True), curve.IsPeriodic())
    assert flags == expected, (flags, expected)
    print(idx, flags)
