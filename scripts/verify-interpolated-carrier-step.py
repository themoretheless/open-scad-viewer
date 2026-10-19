"""Independent STEP validity/basis/topology and numerical retained-volume check.
Numerical quadrature does not replace native original/actual embedding or Gk.
"""
import hashlib
import json
from pathlib import Path
import sys
import math
import OCP
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.TopExp import TopExp_Explorer
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopoDS import TopoDS
from OCP.BRepAdaptor import BRepAdaptor_Surface
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps


def unique(shape, kind):
    items = []
    it = TopExp_Explorer(shape, kind)
    while it.More():
        item = it.Current()
        if not any(item.IsSame(x) for x in items):
            items.append(item)
        it.Next()
    return items


def gauss(order):
    nodes, weights = [], []
    for i in range(1, order+1):
        z = math.cos(math.pi*(i-0.25)/(order+0.5))
        for _ in range(32):
            p0, p1 = 1., z
            for k in range(2, order+1):
                p0, p1 = p1, ((2*k-1)*z*p1-(k-1)*p0)/k
            derivative = order*(z*p1-p0)/(z*z-1)
            change = p1/derivative
            z -= change
            if abs(change) < 1e-15:
                break
        nodes.append(z)
        weights.append(2/((1-z*z)*derivative*derivative))
    for power in range(2*order):
        expected = 2/(power+1) if power % 2 == 0 else 0.
        assert abs(sum(w*x**power for x, w in zip(nodes, weights))-expected) < 1e-12
    return nodes, weights


def basis(knots, degree, t):
    rows = [float(a <= t < b) for a, b in zip(knots, knots[1:])]
    previous = rows
    for p in range(1, degree+1):
        previous = rows
        rows = []
        for i in range(len(knots)-p-1):
            a, b = knots[i+p]-knots[i], knots[i+p+1]-knots[i+1]
            rows.append(((t-knots[i])/a*previous[i] if a else 0.)
                        + ((knots[i+p+1]-t)/b*previous[i+1] if b else 0.))
    derivative = []
    for i in range(len(rows)):
        a, b = knots[i+degree]-knots[i], knots[i+degree+1]-knots[i+1]
        derivative.append((degree/a*previous[i] if a else 0.)
                          - (degree/b*previous[i+1] if b else 0.))
    return rows, derivative


def flux(surface, order):
    nodes, weights = gauss(order)
    jets = []
    for axis in ['U', 'V']:
        p, knots = surface['degree'+axis], surface['knots'+axis]
        lo, hi = knots[p], knots[-p-1]
        jets.append([(*basis(knots, p, (x+1)*(hi-lo)/2+lo), w*(hi-lo)/2)
                     for x, w in zip(nodes, weights)])
    poles, weights = surface['controlPoints'], surface['weights']
    net = [[[v*weights[i][j] for v in [*pole, 1.]] for j, pole in enumerate(row)]
           for i, row in enumerate(poles)]
    total = 0.
    for u, du, wu in jets[0]:
        collapsed = [[sum(u[i]*net[i][j][k] for i in range(len(u)))
                      for k in range(4)] for j in range(len(net[0]))]
        collapsed_u = [[sum(du[i]*net[i][j][k] for i in range(len(u)))
                        for k in range(4)] for j in range(len(net[0]))]
        for v, dv, wv in jets[1]:
            h = [sum(v[j]*collapsed[j][k] for j in range(len(v))) for k in range(4)]
            hu = [sum(v[j]*collapsed_u[j][k] for j in range(len(v))) for k in range(4)]
            hv = [sum(dv[j]*collapsed[j][k] for j in range(len(v))) for k in range(4)]
            point = [x/h[3] for x in h[:3]]
            first = [(hu[k]-point[k]*hu[3])/h[3] for k in range(3)]
            second = [(hv[k]-point[k]*hv[3])/h[3] for k in range(3)]
            density = sum(point[k]*(first[(k+1)%3]*second[(k+2)%3]
                          - first[(k+2)%3]*second[(k+1)%3]) for k in range(3))/3
            total += density*wu*wv
    return total


def expanded(surface, axis):
    return [getattr(surface, axis+'Knot')(i)
            for i in range(1, getattr(surface, 'Nb'+axis+'Knots')()+1)
            for _ in range(getattr(surface, axis+'Multiplicity')(i))]


def main(directory, output):
    body = json.loads((directory/'body.json').read_text())
    restored = json.loads((directory/'restored-model.json').read_text())
    assert body['certified'] and body['idealBodyEmbeddingCertified']
    assert body['idealBody']['volumeRegularityCertified'] and body['idealBody']['parameterTopologyPreserved']
    assert body['seamContinuity'] == 'C0' and not body['smoothnessCertified']
    assert len(restored['faces']) == len(body['model']['faces'])
    model = body['model']
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(directory/'body.step')) == IFSelect_RetDone
    assert reader.TransferRoots() == 1
    shape = reader.OneShape()
    valid = BRepCheck_Analyzer(shape).IsValid()
    solids, faces, edges, vertices = [unique(shape, k) for k in
                                    [TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX]]
    assert valid and len(solids) == 1 and len(faces) == len(model['faces']) == 6
    assert len(edges) == len(model['edges']) and len(vertices) == len(model['vertices'])
    adjacency = [sum(any(e.IsSame(x) for x in unique(f, TopAbs_EDGE)) for f in faces) for e in edges]
    assert all(n == 2 for n in adjacency)
    imported = [BRepAdaptor_Surface(TopoDS.Face_s(f)).BSpline() for f in faces]
    remaining = set(range(len(imported)))
    max_pole_error = 0.
    uses = model['shells'][model['bodies'][0]['outerShell']]['faces']
    reversed_faces = {u['face']: u['reversed'] for u in uses}
    assert len(reversed_faces) == len(model['faces'])
    for face_index, f in enumerate(model['faces']):
        assert not f['holes']
        s = f['surface']
        # This fixture's wall faces occupy the complete natural rectangle;
        # reject any unexpected trim before integrating an untrimmed chart.
        ua, ub = s['knotsU'][s['degreeU']], s['knotsU'][-s['degreeU']-1]
        va, vb = s['knotsV'][s['degreeV']], s['knotsV'][-s['degreeV']-1]
        corners = {(ua, va), (ua, vb), (ub, va), (ub, vb)}
        segments = []
        coedges = model['loops'][f['outer']]['coedges']
        assert len(coedges) == 4
        for coedge in coedges:
            curve = coedge['pcurve']
            assert curve['degree'] == 1 and len(curve['controlPoints']) == 2
            a, b = [tuple(x) for x in curve['controlPoints']]
            assert a in corners and b in corners and a != b
            assert a[0] == b[0] or a[1] == b[1]
            segments.append(frozenset([a, b]))
        assert len(set(segments)) == 4
        # STEP /9 binds exported orientation by reversing U for reversed
        # face uses. Compare the full reparameterized source basis/poles.
        if reversed_faces[face_index]:
            s = dict(s, controlPoints=list(reversed(s['controlPoints'])),
                     weights=list(reversed(s['weights'])),
                     knotsU=[ua+ub-x for x in reversed(s['knotsU'])])
        nu, nv = len(s['controlPoints']), len(s['controlPoints'][0])
        candidates = []
        for i in remaining:
            q = imported[i]
            if [q.UDegree(), q.VDegree(), q.NbUPoles(), q.NbVPoles()] != [s['degreeU'], s['degreeV'], nu, nv]:
                continue
            if expanded(q, 'U') != s['knotsU'] or expanded(q, 'V') != s['knotsV']:
                continue
            err = max(abs(a-b) for u in range(nu) for v in range(nv)
                      for a, b in zip(q.Pole(u+1, v+1).Coord(), s['controlPoints'][u][v]))
            candidates.append((err, i))
        assert candidates, 'No matching independent STEP surface basis'
        err, i = min(candidates)
        assert err < 1e-10, err
        q = imported[i]
        scale = q.Weight(1, 1)/s['weights'][0][0]
        assert max(abs(q.Weight(u+1, v+1)/scale-s['weights'][u][v])
                   for u in range(nu) for v in range(nv)) < 1e-12
        remaining.remove(i)
        max_pole_error = max(max_pole_error, err)
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-12, True)
    volume = props.Mass()
    uses = model['shells'][model['bodies'][0]['outerShell']]['faces']
    reference = [sum((-1 if u['reversed'] else 1)*flux(model['faces'][u['face']]['surface'], n)
                     for u in uses) for n in [12, 20]]
    assert volume > 0 and all(v > 0 for v in reference)
    assert abs(reference[0]-reference[1]) < 1e-8
    assert abs(volume-reference[1]) < 1e-8
    assert abs(volume-3.9375) < 1e-10, (volume, reference, 'Independent source frustum volume differs')
    report = dict(schema='original-interpolated-carrier-independent-occt/1', passed=True, ocpVersion=OCP.__version__,
                  scope='One specified distinct-carrier native STEP body: independent validity, full retained NURBS bases, topology, orientation and converged numerical retained-volume flux. Original embedding/Gk remain separate native proofs.',
                  stepSha256=hashlib.sha256((directory/'body.step').read_bytes()).hexdigest(),
                  nativeBodySha256=hashlib.sha256((directory/'body.json').read_bytes()).hexdigest(),
                  verifierSha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
                  solids=len(solids), faces=len(faces), edges=len(edges), vertices=len(vertices),
                  twoFaceEdges=sum(n == 2 for n in adjacency), maxPoleErrorMm=max_pole_error,
                  volumeMm3=volume, independentFluxVolumeMm3=reference,
                  volumeErrorMm3=abs(volume-reference[1]), fullGoalComplete=False)
    output.write_text(json.dumps(report, indent=2)+'\n')
    print(json.dumps(report))


if __name__ == '__main__':
    main(Path(sys.argv[1]), Path(sys.argv[2]))
