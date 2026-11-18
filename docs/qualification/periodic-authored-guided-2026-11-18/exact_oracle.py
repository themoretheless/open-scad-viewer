"""Qualify the authored axis and guided offset from the native material records."""
from fractions import Fraction as Q
import json
from pathlib import Path
import runpy
import sys

PRIOR = Path(__file__).resolve().parents[1] / "periodic-basis-frenet-2026-11-18"
basis = runpy.run_path(str(PRIOR / "exact_oracle.py"))
span_polynomials = basis["span_polynomials"]
derivative = basis["derivative"]
endpoint = basis["endpoint"]


def load(prefix, logfile):
    lines = [line for line in logfile.splitlines() if line.startswith(prefix)]
    assert len(lines) == 1
    return json.loads(lines[0][len(prefix):])


def verify(logfile):
    a = load("periodic-authored-material-source=", logfile)
    g = load("periodic-guided-material-source=", logfile)
    assert a["path"] == g["path"]
    path = list(span_polynomials(a["path"]))
    guide = list(span_polynomials(g["guide"]))
    axis = list(span_polynomials(a["authoredAxis"]))
    assert len(path) == len(guide) == len(axis) == 8
    assert all(normal == [0, 0, 1] for normal in a["authoredNormal"]["controlPoints"])
    for (h, p), (guide_h, rail), (axis_h, field) in zip(path, guide, axis):
        assert h == guide_h == axis_h
        for coordinate in range(3):
            shift = Q(1, 4) if coordinate == 2 else Q(0)
            expected = [v / h for v in derivative(p[coordinate])]
            expected[0] += shift
            assert expected == field[coordinate]
            offset = [u - v for u, v in zip(rail[coordinate], p[coordinate])]
            assert offset == [shift / 2] + [Q(0)] * (len(offset) - 1)
    for spans, jet_order in ((axis, 3), (guide, 4)):
        for left, right in zip(spans, spans[1:] + spans[:1]):
            for order in range(jet_order + 1):
                assert endpoint(left, 1, order) == endpoint(right, 0, order)
    return {
        "path": basis["verify"](a["path"]),
        "authoredAxis": "P'(t) + (0, 0, 1/4) exactly on every original span",
        "guide": "P(t) + (0, 0, 1/8) exactly on every original span",
        "authoredAxisJoinOrder": 3,
        "guideJoinOrder": 4,
    }


if __name__ == "__main__":
    print(json.dumps(verify(Path(sys.argv[1]).read_text()), indent=2))
