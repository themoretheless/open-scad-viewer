"""Independent OCCT evaluation of native authored linear sphere envelopes.

This is a control regression, not a whole-domain interval certificate or STEP
roundtrip. Natural OCCT faces are rebuilt from the emitted original NURBS nets.
"""
import hashlib
import json
import math
from pathlib import Path
import runpy
import sys
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.GeomLProp import GeomLProp_SLProps

build_surface = runpy.run_path(str(Path(__file__).with_name("verify-cad-circular-support-occt.py")))["surface"]


def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


def cross(a, b):
    return [a[(k + 1) % 3] * b[(k + 2) % 3] - a[(k + 2) % 3] * b[(k + 1) % 3] for k in range(3)]


def norm(a):
    return math.hypot(*a)


def expected(case, index, count, u, v):
    c0, c1 = case["centers"]
    d = [b - a for a, b in zip(c0, c1)]
    h = norm(d)
    tangent = [x / h for x in d]
    guide = case["direction"]
    along = dot(guide, tangent)
    radial = [guide[k] - along * tangent[k] for k in range(3)]
    x = [a / norm(radial) for a in radial]
    y = cross(tangent, x)
    a = (case["radii"][1] - case["radii"][0]) / h
    b = math.sqrt(1 - a * a)
    radius = case["radii"][0] + u * (case["radii"][1] - case["radii"][0])
    center = [c0[k] + u * d[k] for k in range(3)]
    start = case["sweep"] * index / count
    end = case["sweep"] * (index + 1) / count
    mid = (start + end) / 2
    w = math.cos((end - start) / 2)
    basis = [(1-v)**2, 2*v*(1-v), v*v]
    denominator = basis[0] + w * basis[1] + basis[2]
    angle_x = (basis[0]*math.cos(start) + basis[1]*math.cos(mid) + basis[2]*math.cos(end)) / denominator
    angle_y = (basis[0]*math.sin(start) + basis[1]*math.sin(mid) + basis[2]*math.sin(end)) / denominator
    point = [center[k] - radius*a*tangent[k] + radius*b*(angle_x*x[k] + angle_y*y[k]) for k in range(3)]
    return point, center, radius


def main(path):
    payload = json.loads(path.read_text())
    assert payload["schema"] == "cad-linear-canal/1"
    rows = []
    for case in payload["cases"]:
        count = len(case["surfaces"])
        samples = regular = poles = 0
        error = radius_error = normal_sine = 0.0
        valid = True
        for index, data in enumerate(case["surfaces"]):
            geom = build_surface(data)
            maker = BRepBuilderAPI_MakeFace(geom, 0., 1., 0., 1., 1e-7)
            valid = valid and maker.IsDone() and BRepCheck_Analyzer(maker.Face()).IsValid()
            for i in range(21):
                for j in range(41):
                    u, v = i / 20, j / 40
                    target, center, radius = expected(case, index, count, u, v)
                    p = geom.Value(u, v)
                    actual = [p.X(), p.Y(), p.Z()]
                    error = max(error, norm([a-b for a, b in zip(actual, target)]))
                    displacement = [a-b for a, b in zip(actual, center)]
                    radius_error = max(radius_error, abs(norm(displacement)-radius))
                    samples += 1
                    if radius == 0:
                        poles += 1
                        continue
                    props = GeomLProp_SLProps(geom, u, v, 1, 1e-12)
                    assert props.IsNormalDefined(), (index, u, v)
                    n = props.Normal()
                    unit = [n.X(), n.Y(), n.Z()]
                    sine = norm(cross(unit, displacement)) / norm(displacement)
                    normal_sine = max(normal_sine, sine*sine)
                    regular += 1
        passed = valid and error < 1e-10 and radius_error < 1e-10 and normal_sine < 1e-18
        rows.append(dict(radii=case["radii"], sweep=case["sweep"], faces=count,
                         occtFacesValid=valid, samples=samples, regularNormalSamples=regular,
                         poleNormalSamplesSkipped=poles, maxPointErrorMm=error,
                         maxRadiusErrorMm=radius_error, maxNormalSineSquared=normal_sine, passed=passed))
    report = dict(schema="cad-linear-canal-occt/1", sourceSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                  scope="Independent OCCT control evaluation and natural open-face validity; no whole-domain proof, native coedge import, closed body, STEP, or UI acceptance.",
                  passed=all(r["passed"] for r in rows), cases=rows)
    path.with_name("linear-canal-occt-report.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(dict(passed=report["passed"], cases=len(rows), samples=sum(r["samples"] for r in rows),
                          maxRadiusErrorMm=max(r["maxRadiusErrorMm"] for r in rows),
                          maxNormalSineSquared=max(r["maxNormalSineSquared"] for r in rows))))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
