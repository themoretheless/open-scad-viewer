"""Verify the independent U/V flags and pole layout with Open CASCADE."""
import sys
from OCP.IFSelect import IFSelect_RetDone
from OCP.IGESControl import IGESControl_Reader

reader = IGESControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
model = reader.IGESModel()
for offset, periodic_u in [(1, True), (0, False)]:
    patch = model.Entity(model.NbEntities() - offset)
    assert patch.TypeNumber() == 128
    assert (patch.IsClosedU(), patch.IsClosedV()) == (periodic_u, not periodic_u)
    assert (patch.IsPeriodicU(), patch.IsPeriodicV()) == (periodic_u, not periodic_u)
    assert patch.IsPolynomial(True) == (not periodic_u)
    for i in range(4):
        for j in range(2):
            u, v = (i, j) if periodic_u else (j, i)
            x, y = [(0, 0), (3, 0), (0, 2), (0, 0)][i]
            assert patch.Pole(u, v).Coord() == (x, y, j)
            assert patch.Weight(u, v) == (0.5 if periodic_u and i == 1 else 1.0)
    print("Verified periodic axis", "U" if periodic_u else "V")
