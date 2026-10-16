"""Independent OCCT checks for the specified original implicit sphere fixtures.
All exact source, pole/fan, global and G2 proofs belong to the Rust owner.
OCCT measurements here are numerical supplementary interchange evidence.
"""
import hashlib
import json
import math
from pathlib import Path
import sys
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepLProp import BRepLProp_SLProps


def static(owner, name):
    return getattr(owner, name + '_s', None) or getattr(owner, name)


def unique(shape, kind):
    out = []
    it = TopExp_Explorer(shape, kind)
    while it.More():
        item = it.Current()
        if not any(item.IsSame(old) for old in out):
            out.append(item)
        it.Next()
    return out


def check(step):
    source = json.loads(step.with_suffix('.source.json').read_text())['source']
    audit = json.loads(step.with_suffix('.audit.json').read_text())
    assert audit['certified'] and audit['idealSourceBoundaryG2Certified']
    radius, center = source['radius'], source['center']
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(step)) == IFSelect_RetDone and reader.TransferRoots() == 1
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    solids, faces, edges, vertices = [unique(shape, k) for k in [TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX]]
    degenerate = sum(bool(static(BRep_Tool, 'Degenerated')(static(TopoDS, 'Edge')(e))) for e in edges)
    # The STEP reader may split one native point edge into four face-owned
    # degenerate edges. Compare their geometric pole ownership, not raw IDs.
    pole_points = [audit['model']['vertices'][audit['model']['edges'][e]['vertices'][0]]['point']
                   for e in audit['poleEdges']]
    observed_poles = []
    pole_error = 0.
    for edge in edges:
        if not static(BRep_Tool, 'Degenerated')(static(TopoDS, 'Edge')(edge)):
            continue
        ends = unique(edge, TopAbs_VERTEX)
        assert ends and pole_points
        for vertex in ends:
            point = static(BRep_Tool, 'Pnt')(static(TopoDS, 'Vertex')(vertex))
            xyz = [point.X(), point.Y(), point.Z()]
            distances = [max(abs(a-b) for a, b in zip(xyz, pole)) for pole in pole_points]
            closest = min(range(len(distances)), key=distances.__getitem__)
            pole_error = max(pole_error, distances[closest])
            observed_poles.append(closest)
    poles_complete = set(observed_poles) == set(range(len(pole_points)))
    # Independent span-aware Gauss-Kronrod integration, at two accuracies.
    # Require convergence without consulting the analytic reference volume.
    integrations = []
    for eps in [1e-10, 1e-12]:
        props = GProp_GProps()
        error = static(BRepGProp, 'VolumePropertiesGK')(
            shape, props, Eps=eps, OnlyClosed=True, IsUseSpan=True, CGFlag=True)
        centroid = props.CentreOfMass()
        integrations.append(dict(eps=eps, volume=props.Mass(), estimatedError=error,
                                 centroid=[centroid.X(), centroid.Y(), centroid.Z()]))
    volume = integrations[-1]['volume']
    converged = (all(math.isfinite(i['volume']) and math.isfinite(i['estimatedError'])
                     and i['estimatedError'] >= 0 for i in integrations)
                 and abs(integrations[0]['volume']-volume) < 1e-10*max(1., abs(volume)))
    expected_volume = 4. * math.pi * radius**3 / 3.
    center_error = max(abs(x-y) for x, y in zip(integrations[-1]['centroid'], center))
    radial_error, curvature_error, samples = 0., 0., 0
    for face in faces:
        adaptor = BRepAdaptor_Surface(static(TopoDS, 'Face')(face))
        u0, u1 = adaptor.FirstUParameter(), adaptor.LastUParameter()
        v0, v1 = adaptor.FirstVParameter(), adaptor.LastVParameter()
        for a in [0.125, 0.5, 0.875]:
            for b in [0.125, 0.5, 0.875]:
                u, v = u0+(u1-u0)*a, v0+(v1-v0)*b
                p = adaptor.Value(u, v)
                r = math.sqrt(sum((x-y)**2 for x, y in zip([p.X(), p.Y(), p.Z()], center)))
                radial_error = max(radial_error, abs(r-radius))
                local = BRepLProp_SLProps(adaptor, u, v, 2, 1e-10)
                assert local.IsCurvatureDefined()
                curvature_error = max(curvature_error, abs(abs(local.MinCurvature())-1./radius), abs(abs(local.MaxCurvature())-1./radius))
                samples += 1
    passed = (valid and len(solids) == 1 and len(faces) == 8 and len(vertices) == 6
              and len(edges)-degenerate == 12 and poles_complete and pole_error < 1e-8
              and converged
              and volume > 0 and abs(volume-expected_volume) < 1e-6
              and center_error < 1e-8 and radial_error < 1e-8 and curvature_error < 1e-8)
    return dict(name=step.stem, passed=passed, stepSha256=hashlib.sha256(step.read_bytes()).hexdigest(),
                sourceSha256=hashlib.sha256(step.with_suffix('.source.json').read_bytes()).hexdigest(),
                rustAuditSha256=hashlib.sha256(step.with_suffix('.audit.json').read_bytes()).hexdigest(),
                radiusMm=radius, centerMm=center, occtValid=valid, solids=len(solids), faces=len(faces), edges=len(edges), vertices=len(vertices),
                degenerateEdges=degenerate, originalPoleEdges=len(pole_points), observedOriginalPoles=len(set(observed_poles)),
                poleOwnershipErrorMm=pole_error, volumeIntegration=integrations, volumeConverged=converged, quotientEuler=len(vertices)-(len(edges)-degenerate)+len(faces),
                volumeMm3=volume, expectedVolumeMm3=expected_volume, volumeErrorMm3=abs(volume-expected_volume),
                centerErrorMm=center_error, sourceRadialErrorMm=radial_error, curvatureErrorPerMm=curvature_error, sampledPoints=samples)


def main(directory, output):
    cases = [check(p) for p in sorted(directory.glob('*.step'))]
    passed = len(cases) == 4 and all(c['passed'] for c in cases)
    result = dict(schema='original-implicit-sphere-occt/1', ocpVersion=OCP.__version__, passed=passed,
                  scope='Four specified original implicit sphere STEP fixtures: independent OCCT validity, pole-quotient topology, positive volume, center, sampled radius and curvature; exact global/G2 proof remains native.', cases=cases)
    output.write_text(json.dumps(result, indent=2)+'\n')
    print(json.dumps(result))
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main(Path(sys.argv[1]), Path(sys.argv[2])))
