#!/usr/bin/env python3
"""Independent STEP/volume/owned-root checks for the three-face trim control.
Samples supplement native whole-root, trim, graph-selection and closure proofs.
"""
import hashlib
import json
import math
import sys
from pathlib import Path
from OCP.STEPControl import STEPControl_Reader
from OCP.IFSelect import IFSelect_RetDone
from OCP.BRepCheck import BRepCheck_Analyzer
from OCP.BRepGProp import BRepGProp
from OCP.GProp import GProp_GProps
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX, TopAbs_WIRE
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.GeomAPI import GeomAPI_ProjectPointOnSurf
import occt_surface_jets as jets

if not __debug__:
    raise RuntimeError('Qualification requires assertions')
root = Path(sys.argv[1])
m, source, partition, cutter, r = [json.loads((root / name).read_text()) for name in
                                ['model.json', 'source.json', 'partition.json', 'cutter.json', 'manifest.json']]
assert r['stage'] == 'complete' and r['materialRemoved'] and r['noEndpointHints'] and r['oneOriginalRootRegistry']
assert not r['generalBrepClaim'] and not r['g2Claim']
assert r['atomicSharedBudget'] and 0 < r['atomicCells'] <= 100000
assert 0 <= r['atomicExactWork'] <= 1000000 and 0 < r['atomicDomainCells'] <= 100000
for key in ['nativePartitionCells', 'nativeCapCells', 'nativeCapDomainCells', 'visits']:
    assert 0 < r[key] <= 100000
for key in ['nativePartitionExactWork', 'nativeCapExactWork']:
    assert 0 <= r[key] <= 1000000
assert len(set(r['sourceFaces'])) == 3
cap = m['faces'][r['capFace']]
support_json = cutter['faces'][r['cutterFace']]['surface']
assert cap['surface'] == support_json, 'Cap changed the original cutter surface'
assert support_json['degreeU'] == support_json['degreeV'] == 2
assert all(w == 1 for row in support_json['weights'] for w in row)
sq = [1., -1., 1.]
for i, row in enumerate(support_json['controlPoints']):
    for j, p in enumerate(row):
        assert p == [2. * (-1. + i), -1. + j, -0.02 + 0.02 * sq[i] + sq[j]]
assert r['cutEquation'] == dict(constant=-0.02, xSquared=0.005, ySquared=1.)
assert r['bounds'] == dict(x=[-0.5, 0.5], y=[-0.5, 0.5], z=[-0.0625, 0.1875])


def read_step(name):
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(root / name)) == IFSelect_RetDone and reader.TransferRoots()
    shape = reader.OneShape()
    assert BRepCheck_Analyzer(shape).IsValid(), 'Invalid independently loaded STEP'
    return shape


def entities(shape, kind):
    out = []
    it = TopExp_Explorer(shape, kind)
    while it.More():
        x = it.Current()
        if not any(x.IsSame(y) for y in out):
            out.append(x)
        it.Next()
    return out


def volume(shape):
    props = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, props, 1e-11, True, False)
    return props.Mass()


shape = read_step('body.step')
before = read_step('source.step')
counts = {n: len(entities(shape, k)) for n, k in [('solids', TopAbs_SOLID), ('faces', TopAbs_FACE),
                                               ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
assert counts == dict(solids=1, faces=len(m['faces']), edges=len(m['edges']), vertices=len(m['vertices']))
face_holes = sum(len(entities(f, TopAbs_WIRE)) - 1 for f in entities(shape, TopAbs_FACE))
assert counts['vertices'] - counts['edges'] + counts['faces'] - face_holes == 2
# Integrate the specified polynomial cutter over the unchanged box: removed
# material is (4/3) * integral[-.5,.5] (.2075-.005*x*x)^(3/2) dx.
a, b = 0.2075, 0.005
theta = math.asin(0.5 * math.sqrt(b / a))
removed = a * a / (3. * math.sqrt(b)) * (3. * theta + 2. * math.sin(2. * theta) + math.sin(4. * theta) / 4.)
expected = 0.25 - removed
actual = volume(shape)
assert abs(volume(before) - 0.25) < 1e-12
assert abs(actual - expected) < 1e-9, 'Changed material volume differs from the independent integral'
assert actual < volume(before)
# Surviving native topology must preserve the stored source definitions and IDs.
for old, new in enumerate(r['partitionVertexMap']):
    if new is not None:
        assert m['vertices'][new] == partition['vertices'][old]
        assert m['topologyIds']['vertices'][new] == partition['topologyIds']['vertices'][old]
for old, new in enumerate(r['partitionEdgeMap']):
    if new is not None:
        assert m['edges'][new]['curve'] == partition['edges'][old]['curve']
        assert m['edges'][new]['vertices'] == [r['partitionVertexMap'][v] for v in partition['edges'][old]['vertices']]
        assert m['topologyIds']['edges'][new] == partition['topologyIds']['edges'][old]
for v in range(len(source['vertices'])):
    new = r['partitionVertexMap'][v]
    assert new is not None and m['vertices'][new] == source['vertices'][v]
    assert m['topologyIds']['vertices'][new] == source['topologyIds']['vertices'][v]
step_edges = entities(shape, TopAbs_EDGE)
step_vertices = entities(shape, TopAbs_VERTEX)
step_faces = entities(shape, TopAbs_FACE)
expected_roots = {r['partitionVertexMap'][v] for v in range(len(source['vertices']), len(partition['vertices']))
                  if sum(v in e['vertices'] for e in partition['edges']) == 4}
assert None not in expected_roots
assert set(r['sharedBoundaryVertices']) == expected_roots and len(expected_roots) == 4
for v in r['sharedBoundaryVertices']:
    matches = [sv for sv in step_vertices if math.dist(BRep_Tool.Pnt_s(TopoDS.Vertex_s(sv)).Coord(), m['vertices'][v]['point']) < 1e-9]
    assert len(matches) == 1, 'A shared original root split into multiple STEP vertices'
    sv = matches[0]
    assert sum(any(x.IsSame(sv) for x in entities(e, TopAbs_VERTEX)) for e in step_edges) == 3
    assert sum(any(x.IsSame(sv) for x in entities(f, TopAbs_VERTEX)) for f in step_faces) == 3
contact_edges = [c['edge'] for w in [cap['outer']] + cap['holes'] for c in m['loops'][w]['coedges']]
assert sorted(contact_edges) == sorted(r['contactEdges']) and len(contact_edges) == len(set(contact_edges))
step_curves = [BRepAdaptor_Curve(TopoDS.Edge_s(e)) for e in step_edges]
support = jets.surface(support_json)
maximum_lift = maximum_cutter = maximum_step = 0.
samples = 0


def at(c, t):
    return c.Value(c.FirstParameter() + t * (c.LastParameter() - c.FirstParameter()))


for e in contact_edges:
    curve = jets.curve(m['edges'][e]['curve'])
    matches = [(sc, reverse) for sc in step_curves for reverse in [False, True]
               if all(at(curve, t).Distance(at(sc, 1. - t if reverse else t)) <= 1e-9 for t in [0., 0.5, 1.])]
    assert len(matches) == 1, 'Exported contact curve differs from the saved original'
    sc, reverse = matches[0]
    uses = [(f, c) for f in m['faces'] for w in [f['outer']] + f['holes']
            for c in m['loops'][w]['coedges'] if c['edge'] == e]
    assert len(uses) == 2
    for i in range(17):
        t = i / 16.
        point = at(curve, t)
        maximum_step = max(maximum_step, point.Distance(at(sc, 1. - t if reverse else t)))
        projected = GeomAPI_ProjectPointOnSurf(point, support)
        assert projected.NbPoints()
        maximum_cutter = max(maximum_cutter, projected.LowerDistance())
        for face, c in uses:
            uv = at(jets.curve(c['pcurve']), 1. - t if c['reversed'] else t)
            lifted = jets.surface(face['surface']).Value(uv.X(), uv.Y())
            maximum_lift = max(maximum_lift, point.Distance(lifted))
        samples += 1
assert maximum_step <= 1e-9 and max(maximum_lift, maximum_cutter) <= source['toleranceMm']
report = dict(passed=True, topology=counts, expectedVolumeMm3=expected, actualVolumeMm3=actual,
              volumeErrorMm3=abs(actual-expected), atomicNativeCells=r['atomicCells'], atomicNativeExactWork=r['atomicExactWork'], removedVolumeMm3=volume(before)-actual,
              sharedOriginalRoots=4, contactEdges=len(contact_edges), samples=samples,
              maximumSavedToStepErrorMm=maximum_step, maximumOriginalLiftErrorMm=maximum_lift,
              maximumActualCutterDistanceMm=maximum_cutter, survivingDefinitionsAndIdentitiesPreserved=True,
              materialRemoved=True, globalG2Claim=False, generalBrepClaim=False,
              scope='three-face automatic original-contour trim and polynomial cutter cap',
              sha256={p.name: hashlib.sha256(p.read_bytes()).hexdigest() for p in root.iterdir()
                      if p.suffix in ['.json', '.step'] and p.name != 'independent-check.json'})
(root / 'independent-check.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
