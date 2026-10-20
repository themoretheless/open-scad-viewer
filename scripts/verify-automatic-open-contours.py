#!/usr/bin/env python3
"""Independent STEP/lift checks; samples supplement native whole-product proofs."""
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
from OCP.GeomAPI import GeomAPI_ProjectPointOnSurf
from OCP.BRepAdaptor import BRepAdaptor_Curve
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
import occt_surface_jets as jets

if not __debug__:
    raise RuntimeError('Qualification requires Python assertions')
root = Path(sys.argv[1])
m = json.loads((root / 'model.json').read_text())
source = json.loads((root / 'source.json').read_text())
cutter = json.loads((root / 'cutter.json').read_text())
r = json.loads((root / 'manifest.json').read_text())
assert r['stage'] == 'complete' and r['noEndpointHints']
assert r['originalProductAndSavedCoverage'] or r.get('completeTrimmedCoverage') is True
assert r['cells'] <= 100000 and r['exactWork'] <= 1000000 and r['visits'] <= 100000 and r['evaluations'] <= 4096
assert not r['materialRemoved'] and not r['generalTrimmedContourClaim']
def entities(kind, on=None):
    items = []
    it = TopExp_Explorer(shape if on is None else on, kind)
    while it.More():
        x = it.Current()
        if not any(x.IsSame(y) for y in items):
            items.append(x)
        it.Next()
    return items
source_volume = None
step_validated = bool(m['bodies'])
if step_validated:
    reader = STEPControl_Reader()
    assert reader.ReadFile(str(root / 'body.step')) == IFSelect_RetDone and reader.TransferRoots()
    shape = reader.OneShape()
    assert BRepCheck_Analyzer(shape).IsValid()
    counts = {name: len(entities(kind)) for name, kind in [('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    assert counts == dict(solids=len(m['bodies']), faces=len(m['faces']), edges=len(m['edges']), vertices=len(m['vertices']))
    g = GProp_GProps()
    BRepGProp.VolumeProperties_s(shape, g, 1e-10, True, False)
    if m['bodies']:
        assert abs(g.Mass() - r.get('expectedVolumeMm3', 0.25)) < 1e-10
    if (root / 'source.step').exists():
        before = STEPControl_Reader()
        assert before.ReadFile(str(root / 'source.step')) == IFSelect_RetDone and before.TransferRoots()
        assert BRepCheck_Analyzer(before.OneShape()).IsValid()
        original_mass = GProp_GProps()
        BRepGProp.VolumeProperties_s(before.OneShape(), original_mass, 1e-10, True, False)
        source_volume = original_mass.Mass()
        assert abs(source_volume - g.Mass()) < 1e-10
else:
    assert r.get('stepExported') is False
    counts = None
    g = None
shared_roots = None
if r.get('holeNetwork'):
    assert step_validated
    bound = json.loads((root / 'bound-source.json').read_text())
    for key in ['vertices', 'edges']:
        assert m[key][:len(bound[key])] == bound[key]
        assert m['topologyIds'][key][:len(bound[key])] == bound['topologyIds'][key]
    roots = [v for pair in r['branchEndpoints'] for v in pair]
    assert len(roots) == len(set(roots))
    for v in roots:
        matches = [sv for sv in entities(TopAbs_VERTEX)
                   if math.dist(BRep_Tool.Pnt_s(TopoDS.Vertex_s(sv)).Coord(), m['vertices'][v]['point']) < 1e-9]
        assert len(matches) == 1, 'Original root is not one shared STEP vertex'
        sv = matches[0]
        assert sum(any(x.IsSame(sv) for x in entities(TopAbs_VERTEX, e)) for e in entities(TopAbs_EDGE)) == 3
        assert sum(any(x.IsSame(sv) for x in entities(TopAbs_VERTEX, f)) for f in entities(TopAbs_FACE)) == 3
    shared_roots = len(roots)
surface_euler = None
if 'holeCount' in r:
    assert len(source['faces'][r['sourceFace']]['holes']) == r['holeCount']
    assert shared_roots == 4 * (r['holeCount'] + 1)
    assert len(r['branchEdges']) == 2 * (r['holeCount'] + 1)
    assert len(r['faces']) == r['holeCount'] + 3
    assert all(not m['faces'][f]['holes'] for f in r['faces'])
    face_holes = sum(len(entities(TopAbs_WIRE, f)) - 1 for f in entities(TopAbs_FACE))
    surface_euler = counts['vertices'] - counts['edges'] + counts['faces'] - face_holes
    assert surface_euler == 2 - 2 * r['holeCount'], 'Actual STEP lost a through hole'
regions = r.get('sourceRegions', [dict(sourceFace=r.get('sourceFace'), faces=r['faces'])])
assert all(region['sourceFace'] is not None for region in regions)
assert len({region['sourceFace'] for region in regions}) == len(regions)
assert sorted(f for region in regions for f in region['faces']) == sorted(r['faces'])
for region in regions:
    original_surface = source['faces'][region['sourceFace']]['surface']
    assert all(m['faces'][f]['surface'] == original_surface for f in region['faces'])
if 'sharedBoundaryVertices' in r:
    assert step_validated
    for v in r['sharedBoundaryVertices']:
        assert v >= len(source['vertices'])
        matches = [sv for sv in entities(TopAbs_VERTEX)
                   if math.dist(BRep_Tool.Pnt_s(TopoDS.Vertex_s(sv)).Coord(), m['vertices'][v]['point']) < 1e-9]
        assert len(matches) == 1, 'Adjacent original faces duplicated a shared STEP root'
        sv = matches[0]
        assert sum(any(x.IsSame(sv) for x in entities(TopAbs_VERTEX, e)) for e in entities(TopAbs_EDGE)) == 4
        assert sum(any(x.IsSame(sv) for x in entities(TopAbs_VERTEX, f)) for f in entities(TopAbs_FACE)) == 4
for i, v in enumerate(source['vertices']):
    assert m['vertices'][i] == v and m['topologyIds']['vertices'][i] == source['topologyIds']['vertices'][i]
support = jets.surface(cutter['faces'][r['cutterFace']]['surface'])
maximum_lift = maximum_cutter = maximum_step = 0.
step_curves = [BRepAdaptor_Curve(TopoDS.Edge_s(e)) for e in entities(TopAbs_EDGE)] if step_validated else []
def at(c, t):
    return c.Value(c.FirstParameter() + (c.LastParameter() - c.FirstParameter()) * t)
samples = 0
if r.get('completeTrimmedCoverage'):
    assert r['endpointsStrictlyInsideOriginalChart'] and r['rejectedCandidates'] > 0
    assert len(r['branchEdges']) == len(r['branchEndpoints'])
    for chain, endpoints in zip(r['branchEdges'], r['branchEndpoints']):
        owners = [region for region in regions if any(c['edge'] == chain[0]
                  for f in region['faces'] for w in [m['faces'][f]['outer']] + m['faces'][f]['holes']
                  for c in m['loops'][w]['coedges'])]
        assert len(owners) == 1, 'Saved chain has no unique original support'
        region = owners[0]
        surface = source['faces'][region['sourceFace']]['surface']
        natural = [(surface['knotsU'][surface['degreeU']], surface['knotsU'][len(surface['controlPoints'])]),
                   (surface['knotsV'][surface['degreeV']], surface['knotsV'][len(surface['controlPoints'][0])])]
        for v in endpoints:
            starts = [c for f in region['faces'] for w in [m['faces'][f]['outer']] + m['faces'][f]['holes']
                      for c in m['loops'][w]['coedges'] if m['edges'][c['edge']]['vertices'][int(c['reversed'])] == v]
            assert starts and all(all(lo < u < hi for u, (lo, hi) in zip(c['pcurve']['controlPoints'][0], natural)) for c in starts)

edge_ids = [e for chain in r['branchEdges'] for e in chain]
assert len(edge_ids) == len(set(edge_ids))
if not r.get('completeTrimmedCoverage'):
    assert len(r['branchEdges']) == r['components']
else:
    assert 1 <= len(r['branchEdges']) <= 128
for edges, endpoints in zip(r['branchEdges'], r['branchEndpoints']):
    assert len(set(endpoints)) == 2
    for v in endpoints:
        assert sum(v in m['edges'][e]['vertices'] for e in edges) == 1
    for e in edges:
        curve = jets.curve(m['edges'][e]['curve'])
        exported = []
        if step_validated:
            for sc in step_curves:
                for reverse in [False, True]:
                    if all(at(curve, t).Distance(at(sc, 1-t if reverse else t)) <= 1e-9 for t in [0., 0.5, 1.]):
                        exported.append((sc, reverse))
            assert len(exported) == 1, 'Actual STEP curve differs from saved contour'

        uses = [(f, c) for f in r['faces'] for w in [m['faces'][f]['outer']] + m['faces'][f]['holes'] for c in m['loops'][w]['coedges'] if c['edge'] == e]
        assert len(uses) == 2
        for i in range(17):
            t = i / 16
            point = curve.Value(curve.FirstParameter() + (curve.LastParameter() - curve.FirstParameter()) * t)
            if exported:
                sc, reverse = exported[0]
                maximum_step = max(maximum_step, point.Distance(at(sc, 1-t if reverse else t)))
            projection = GeomAPI_ProjectPointOnSurf(point, support)
            assert projection.NbPoints() > 0
            maximum_cutter = max(maximum_cutter, projection.LowerDistance())
            for f, c in uses:
                pc = jets.curve(c['pcurve'])
                q = 1 - t if c['reversed'] else t
                uv = pc.Value(pc.FirstParameter() + (pc.LastParameter() - pc.FirstParameter()) * q)
                actual = jets.surface(m['faces'][f]['surface']).Value(uv.X(), uv.Y())
                maximum_lift = max(maximum_lift, point.Distance(actual))
            samples += 1
assert maximum_lift <= source['toleranceMm'] and maximum_cutter <= source['toleranceMm'], 'Saved contour leaves original supports'
assert maximum_step <= 1e-9
report = dict(passed=True, surfaceEulerCharacteristic=surface_euler, sourceVolumeMm3=source_volume, volumeChangeMm3=g.Mass()-source_volume if source_volume is not None else None, sharedOriginalBoundaryRoots=shared_roots, adjacentSharedRoots=len(r.get('sharedBoundaryVertices', [])), originalSourceFaces=len(regions), maximumSavedToStepErrorMm=maximum_step if step_validated else None, topology=counts, volumeMm3=g.Mass() if step_validated else None, stepValidated=step_validated, components=r['components'], savedContourEdges=len(edge_ids), samples=samples,
              maximumOriginalFaceLiftErrorMm=maximum_lift, maximumActualCutterDistanceMm=maximum_cutter,
              originalVerticesAndIdentitiesPreserved=True, materialRemoved=False, globalG2Claim=False,
              scope='actual automatic open contour partition; samples supplement native complete root coverage',
              sha256={n: hashlib.sha256((root / n).read_bytes()).hexdigest() for n in ['model.json', 'source.json', 'cutter.json', 'manifest.json'] + (['body.step'] if step_validated else []) + (['source.step'] if (root / 'source.step').exists() else [])})
(root / 'independent-check.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
