"""Check all original span tangent planes of the emitted hollow graph fixture."""
from fractions import Fraction as Q
from pathlib import Path
import json
import sys

prefix = "original-cap-wall-source="
cases = [json.loads(line[len(prefix):]) for line in Path(sys.argv[1]).read_text().splitlines()
         if line.startswith(prefix)]
assert len(cases) == 1
case = cases[0]
assert case["path"]["degree"] == 1
assert case["path"]["weights"] == [1, 1]
assert case["path"]["controlPoints"] == [[0, 0, 0], [0, 0, 2]]
for name, value in (("scale", [1, 0, 0]), ("twist", [0, 0, 0]),
                    ("axis", [0, 0, 1]), ("normal", [1, 0, 0])):
    assert all(p == value for p in case[name]["controlPoints"])
q = [Q(0), Q(-9, 16), Q(5, 4), Q(-9, 16), Q(0)]
by_end = []
for end, cap in enumerate(case["caps"]):
    s = cap["surface"]
    assert s["degreeU"] == s["degreeV"] == 4
    assert len({Q(w) for row in s["weights"] for w in row}) == 1
    for i, row in enumerate(s["controlPoints"]):
        for j, p in enumerate(row):
            assert list(map(Q, p)) == [Q(i, 4), Q(j, 4), end * q[i] * q[j] / 64]
    # q(t) = 12 t(t-1/4)(t-3/4)(t-1). Each span holds one
    # coordinate at a root, so the two caps match along the full span.
    roots = {Q(0), Q(1, 4), Q(3, 4), Q(1)}
    counts = []
    for ring in cap["trims"]:
        spans = 0
        for trim in ring:
            assert trim["degree"] == 1 and len(set(trim["weights"])) == 1
            a, b = [list(map(Q, point)) for point in trim["controlPoints"]]
            assert a != b
            assert any(
                a[k] == b[k] and (a[k] - Q(knots[4])) / (Q(knots[5]) - Q(knots[4])) in roots
                for k, knots in enumerate((s["knotsU"], s["knotsV"]))
            )
            spans += 1
        counts.append(spans)
    # The normalized cap derivatives have XY columns (1,0),(0,1).
    # The common boundary has F_t = (0,0,2). Therefore every cap
    # frame with the wall traversal column has determinant 2.
    by_end.append({"cap": end, "outerSpans": counts[0], "holeSpans": counts[1],
                   "determinantWithWallTraversal": "2"})
print(json.dumps(by_end, indent=2))
