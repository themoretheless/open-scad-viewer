"""Independent OCCT cap/shaft evaluation, sewing and analytical volume control.

Native fixtures remain open support models. An independently sewn OCCT solid
is evidence for these controls, not native source-body or STEP admission.
"""
import hashlib
import json
import math
from pathlib import Path
import runpy
import sys
from OCP.BRepBuilderAPI import BRepBuilderAPI_MakeFace, BRepBuilderAPI_Sewing, BRepBuilderAPI_MakeSolid
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepLib import BRepLib
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.GeomLProp import GeomLProp_SLProps
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SHELL
from OCP.TopoDS import TopoDS

helpers = runpy.run_path(str(Path(__file__).with_name("verify-linear-canal-occt.py")))
build_surface = helpers["build_surface"]
shaft_expected = helpers["expected"]
dot, cross, norm = helpers["dot"], helpers["cross"], helpers["norm"]


def expected(case, part, u, v):
    if part["kind"] == "shaft":
        point, center, radius = shaft_expected(case, part["angular"], part["angularCount"], u, v)
        return point, center, radius, radius == 0
    c0, c1 = case["centers"]
    delta = [b-a for a, b in zip(c0, c1)]
    speed = norm(delta)
    tangent = [d/speed for d in delta]
    along = dot(case["direction"], tangent)
    radial = [case["direction"][k] - along*tangent[k] for k in range(3)]
    x = [d/norm(radial) for d in radial]
    y = cross(tangent, x)
    a = (case["radii"][1]-case["radii"][0])/speed
    join = math.acos(a)
    end = int(part["kind"] == "finish")
    lo, hi = (0., join) if end == 0 else (join, math.pi)
    lower = lo + (hi-lo)*part["meridian"]/part["meridianCount"]
    upper = lo + (hi-lo)*(part["meridian"]+1)/part["meridianCount"]
    middle = (lower+upper)/2
    weight = math.cos((upper-lower)/2)
    basis = [(1-u)**2, 2*u*(1-u), u*u]
    denominator = basis[0] + weight*basis[1] + basis[2]
    rho = sum(b*math.sin(t) for b, t in zip(basis, [lower, middle, upper]))/denominator
    z = -sum(b*math.cos(t) for b, t in zip(basis, [lower, middle, upper]))/denominator
    start = case["sweep"]*part["angular"]/part["angularCount"]
    finish = case["sweep"]*(part["angular"]+1)/part["angularCount"]
    mid = (start+finish)/2
    w = math.cos((finish-start)/2)
    vb = [(1-v)**2, 2*v*(1-v), v*v]
    vd = vb[0]+w*vb[1]+vb[2]
    ax = sum(b*math.cos(t) for b, t in zip(vb, [start, mid, finish]))/vd
    ay = sum(b*math.sin(t) for b, t in zip(vb, [start, mid, finish]))/vd
    center, radius = case["centers"][end], case["radii"][end]
    point = [center[k]+radius*(z*tangent[k]+rho*(ax*x[k]+ay*y[k])) for k in range(3)]
    pole = (end == 0 and part["meridian"] == 0 and u == 0) or (end == 1 and part["meridian"]+1 == part["meridianCount"] and u == 1)
    return point, center, radius, pole


def analytical_volume(case):
    h = norm([b-a for a, b in zip(*case["centers"])])
    r0, r1 = case["radii"]
    a = (r1-r0)/h
    b2 = 1-a*a
    cone = math.pi*h*b2*b2*(r0*r0+r0*r1+r1*r1)/3
    h0, h1 = r0*(1-a), r1*(1+a)
    return cone+math.pi*h0*h0*(r0-h0/3)+math.pi*h1*h1*(r1-h1/3)


def main(path):
    payload = json.loads(path.read_text())
    assert payload["schema"] == "cad-canal-caps/1"
    rows = []
    for case in payload["cases"]:
        sewing = BRepBuilderAPI_Sewing(1e-7)
        point_error = radius_error = sine_squared = 0.
        samples = skipped = 0
        face_valid = True
        for part in case["components"]:
            surface = build_surface(part["surface"])
            maker = BRepBuilderAPI_MakeFace(surface, 0., 1., 0., 1., 1e-7)
            assert maker.IsDone()
            face_valid = face_valid and BRepCheck_Analyzer(maker.Face()).IsValid()
            sewing.Add(maker.Face())
            for i in range(11):
                for j in range(17):
                    u, v = i/10, j/16
                    target, center, radius, pole = expected(case, part, u, v)
                    p = surface.Value(u, v)
                    actual = [p.X(), p.Y(), p.Z()]
                    point_error = max(point_error, norm([a-b for a, b in zip(actual, target)]))
                    displacement = [a-b for a, b in zip(actual, center)]
                    radius_error = max(radius_error, abs(norm(displacement)-radius))
                    samples += 1
                    if pole:
                        skipped += 1
                        continue
                    props = GeomLProp_SLProps(surface, u, v, 1, 1e-12)
                    assert props.IsNormalDefined()
                    n = props.Normal()
                    sine = norm(cross([n.X(), n.Y(), n.Z()], displacement))/norm(displacement)
                    sine_squared = max(sine_squared, sine*sine)
        sewing.Perform()
        shape = sewing.SewedShape()
        explorer = TopExp_Explorer(shape, TopAbs_SHELL)
        shells = []
        while explorer.More():
            shells.append(TopoDS.Shell(explorer.Current()))
            explorer.Next()
        assert len(shells) == 1, len(shells)
        solid_maker = BRepBuilderAPI_MakeSolid(shells[0])
        assert solid_maker.IsDone()
        solid = solid_maker.Solid()
        oriented = BRepLib.OrientClosedSolid_s(solid)
        valid = BRepCheck_Analyzer(solid).IsValid()
        properties = GProp_GProps()
        integration_error = BRepGProp.VolumePropertiesGK_s(solid, properties, Eps=1e-12, OnlyClosed=True, IsUseSpan=True)
        volume = properties.Mass()
        reference = analytical_volume(case)
        volume_error = abs(volume-reference)
        passed = 0 <= integration_error < 1e-10 and face_valid and oriented and valid and sewing.NbFreeEdges() == 0 and sewing.NbMultipleEdges() == 0 and point_error < 1e-10 and radius_error < 1e-10 and sine_squared < 1e-18 and volume_error < 1e-7
        rows.append(dict(radii=case["radii"], sweep=case["sweep"], faces=len(case["components"]), samples=samples,
                         skippedPoleNormals=skipped, naturalFacesValid=face_valid, occtSolidValid=valid,
                         occtSolidOriented=oriented, freeEdges=sewing.NbFreeEdges(), multipleEdges=sewing.NbMultipleEdges(),
                         maxPointErrorMm=point_error, maxRadiusErrorMm=radius_error, maxNormalSineSquared=sine_squared,
                         integrationRelativeError=integration_error, volumeMm3=volume, analyticalVolumeMm3=reference, volumeErrorMm3=volume_error, passed=passed))
    report = dict(schema="cad-canal-caps-occt/1", sourceSha256=hashlib.sha256(path.read_bytes()).hexdigest(),
                  scope="Independent OCCT surface control samples, natural-face validity, sewing and analytical volume; no native source pole/embedding/body, STEP, or UI admission.",
                  passed=all(row["passed"] for row in rows), cases=rows)
    path.with_name("canal-cap-occt-report.json").write_text(json.dumps(report, indent=2)+"\n")
    print(json.dumps(dict(passed=report["passed"], cases=len(rows), samples=sum(r["samples"] for r in rows),
                          maxRadiusErrorMm=max(r["maxRadiusErrorMm"] for r in rows), maxVolumeErrorMm3=max(r["volumeErrorMm3"] for r in rows))))
    return 0 if report["passed"] else 1


if __name__ == "__main__":
    sys.exit(main(Path(sys.argv[1])))
