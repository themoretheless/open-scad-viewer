"""Read the complete spinner in OCCT and compare its 18 placed local frames."""
import math
import sys
from OCP.IFSelect import IFSelect_RetDone
from OCP.STEPControl import STEPControl_Reader
from OCP.TopAbs import TopAbs_SOLID
from OCP.TopExp import TopExp_Explorer

reader = STEPControl_Reader()
assert reader.ReadFile(sys.argv[1]) == IFSelect_RetDone
assert reader.TransferRoots() == 1
it = TopExp_Explorer(reader.OneShape(), TopAbs_SOLID)
central = 0
planets = {}
while it.More():
    frame = it.Current().Location().Transformation()
    x, y, z = (frame.Value(i, 4) for i in range(1, 4))
    if math.hypot(x, y) < 1e-9:
        central += 1
    else:
        slot = round(math.degrees(math.atan2(y, x)) / 20) % 18
        assert slot not in planets
        orbit = math.radians(20 * slot)
        angle = 9 * orbit + math.radians(135)
        c, s = math.cos(angle), math.sin(angle)
        expected = [(c, -s, 0, 27*math.cos(orbit)),
                    (s, c, 0, 27*math.sin(orbit)), (0, 0, 1, 0)]
        planets[slot] = max(abs(frame.Value(i+1,j+1) - value)
                            for i,row in enumerate(expected) for j,value in enumerate(row))
    it.Next()
assert central == 2 and len(planets) == 18
assert max(planets.values()) < 1e-9
print(f"Two central solids, 18 distinct planet frames, maximum matrix error {max(planets.values()):.3g}")
