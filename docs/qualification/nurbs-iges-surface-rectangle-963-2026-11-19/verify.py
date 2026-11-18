"""Compare type-128 source and Rust-restricted surface geometry using OCCT."""
import sys
from pathlib import Path
from OCP.IFSelect import IFSelect_RetDone
from OCP.IGESControl import IGESControl_Reader
from OCP.IGESToBRep import IGESToBRep_BasicSurface

for name, left, right in [("band", 0, 3), ("patch", 0.5, 2.25)]:
    surfaces = []
    for role in ("source", "trimmed"):
        reader = IGESControl_Reader()
        path = Path(sys.argv[1]) / f"{name}-{role}.igs"
        assert reader.ReadFile(str(path)) == IFSelect_RetDone
        model = reader.IGESModel()
        entity = model.Entity(model.NbEntities())
        assert entity.TypeNumber() == 128
        # IGES 5.3 section 4.24 specifies U0,U1,V0,V1. Read the physical
        # fields independently; this OCCT reader interchanges the middle two.
        owner = 2 * model.NbEntities() - 1
        payload = "".join(row[:64].rstrip() for row in path.read_text().splitlines()
                          if row[72] == "P" and int(row[64:72]) == owner)
        bounds = tuple(map(float, payload.rstrip(";").split(",")[-4:]))
        assert bounds == (left, right, 0.25, 0.75)

        if role == "trimmed":
            assert entity.IsPeriodicU() == (name == "band")
        surfaces.append(IGESToBRep_BasicSurface().TransferBSplineSurface(entity))
    maximum = 0.
    for i in range(33):
        for j in range(17):
            u, v = left + (right-left)*i/32, 0.25 + j/32
            maximum = max(maximum, surfaces[0].Value(u,v).Distance(surfaces[1].Value(u,v)))
    assert maximum < 1e-12, maximum
    print(name, maximum)
