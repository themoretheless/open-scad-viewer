#!/usr/bin/env python3
"""Independent rational-root partition and saved cap STEP controls."""
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
from OCP.TopAbs import TopAbs_SOLID, TopAbs_FACE, TopAbs_EDGE, TopAbs_VERTEX
from OCP.TopExp import TopExp_Explorer
from OCP.TopoDS import TopoDS
from OCP.BRep import BRep_Tool
from occt_surface_jets import surface, curve

root = Path(sys.argv[1])
automatic = '--automatic' in sys.argv[2:]
read = lambda name: json.loads((root / name).read_text())
meta = read('manifest.json')
model, source, bound, cutter = map(read, ['partition.json', 'source.json', 'bound-source.json', 'cutter.json'])
traces = read('native-traces.json')
assert meta['partitionOnly'] and not meta['replacementQualified'] and not meta['arbitraryBrepQualified']
assert meta['networkCells'] <= 100000 and meta['networkExactWork'] <= 1000000
assert len(traces) == meta['savedTraceCount'] and 1 <= len(traces) <= 1024
weights = cutter['faces'][0]['surface']['weights']
assert all(w > 0 for row in weights for w in row) and len({w for row in weights for w in row}) > 1

def entities(shape, kind):
    result = []
    it = TopExp_Explorer(shape, kind)
    while it.More():
        item = it.Current()
        if not any(item.IsSame(other) for other in result):
            result.append(item)
        it.Next()
    return result

reader = STEPControl_Reader()
assert reader.ReadFile(str(root / 'partition.step')) == IFSelect_RetDone and reader.TransferRoots()
shape = reader.OneShape()
assert not shape.IsNull() and BRepCheck_Analyzer(shape).IsValid()
topology = {name: len(entities(shape, kind)) for name, kind in [
    ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
assert topology == dict(solids=1, faces=len(model['faces']), edges=len(model['edges']), vertices=len(model['vertices']))
assert topology['faces'] == 10
# Source is the actual cube; subdivision must leave its physical solid unchanged.
points = [v['point'] for v in source['vertices']]
assert len(points) == 8 and {tuple(p) for p in points} == {
    (x, y, z) for x in [-0.5, 0.5] for y in [-0.5, 0.5] for z in [-0.05, 0.2]}
props = GProp_GProps()
BRepGProp.VolumeProperties_s(shape, props, 1e-10, True, False)
volume_error = abs(props.Mass() - 0.25)
assert volume_error < 1e-9

maximum = 0.
samples = 0
saved_indices = set()
for trace in traces:
    assert trace['secondSurface'] == cutter['faces'][0]['surface']
    assert any(f['surface'] == trace['firstSurface'] for f in source['faces'])
    matching = [i for i, e in enumerate(model['edges']) if e['curve'] == trace['curve']]
    assert len(matching) == 1 and matching[0] not in saved_indices
    edge_index = matching[0]
    saved_indices.add(edge_index)
    uses = [(f, c) for f in model['faces'] for w in [f['outer'], *f['holes']]
            for c in model['loops'][w]['coedges'] if c['edge'] == edge_index]
    assert len(uses) == 2 and all(f['surface'] == trace['firstSurface'] for f, _ in uses)
    tol = source['toleranceMm'] / 2
    assert 0 <= trace['worldErrorUpperMm'] <= tol
    world = curve(trace['curve'])
    supports = [surface(trace['firstSurface']), surface(trace['secondSurface'])]
    pcs = list(map(curve, trace['pcurves']))
    for i in range(65):
        t = i / 64
        point = world.Value(world.FirstParameter() + t * (world.LastParameter() - world.FirstParameter()))
        for s, pc in zip(supports, pcs):
            uv = pc.Value(pc.FirstParameter() + t * (pc.LastParameter() - pc.FirstParameter()))
            actual = s.Value(uv.X(), uv.Y())
            maximum = max(maximum, math.dist(point.Coord(), actual.Coord()))
        samples += 1
assert maximum <= source['toleranceMm'] / 2
# Original shared root points survive the independent STEP topology exactly once.
roots = bound['vertices'][8:]
assert len(roots) == 4
step_vertices = entities(shape, TopAbs_VERTEX)
for root_vertex in roots:
    matches = [v for v in step_vertices if math.dist(root_vertex['point'], BRep_Tool.Pnt_s(TopoDS.Vertex_s(v)).Coord()) < 1e-9]
    assert len(matches) == 1
    vertex = matches[0]
    assert sum(any(vertex.IsSame(v) for v in entities(e, TopAbs_VERTEX)) for e in entities(shape, TopAbs_EDGE)) == 4
    assert sum(any(vertex.IsSame(v) for v in entities(f, TopAbs_VERTEX)) for f in entities(shape, TopAbs_FACE)) == 4

report = dict(passed=True, topology=topology, volumeMm3=props.Mass(), volumeErrorMm3=volume_error,
              sharedSavedEdges=len(saved_indices), originalRootVerticesShared=True,
              sampledSupportSites=samples, maximumSampledSupportDeviationMm=maximum,
              fileSha256={name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in
                          ['source.json', 'cutter.json', 'partition.json', 'partition.step', 'native-traces.json']},
              scope='actual nonconstant-weight rational supports: root-owned subdivision preserves the source solid; native whole-piece proofs plus independent support samples; material selection is checked separately; general fillets and global embedding remain unqualified')
if (root / 'cap-manifest.json').exists():
    cap_meta = read('cap-manifest.json')
    assert cap_meta['explicitSeedCapQualified'] and not cap_meta['filletQualified']
    if automatic:
        assert cap_meta['automaticMaterialSelectionQualified'] and cap_meta['automaticProjectedSelectionQualified']
        assert cap_meta['automaticProjectionCells'] == cap_meta['replacementCells']
        assert cap_meta['automaticCoefficientWork'] <= 2000000 and 0 < cap_meta['automaticDomainCells'] <= 100000
    assert meta['networkCells'] + cap_meta['replacementCells'] <= 100000
    assert meta['networkExactWork'] + cap_meta['replacementExactWork'] <= 1000000
    reference = read('expected-rational-cap-volume.json')
    assert reference['cutterSha256'] == hashlib.sha256((root / 'cutter.json').read_bytes()).hexdigest()
    capped = read('model.json')
    cap_reader = STEPControl_Reader()
    assert cap_reader.ReadFile(str(root / 'body.step')) == IFSelect_RetDone and cap_reader.TransferRoots()
    cap_shape = cap_reader.OneShape()
    assert not cap_shape.IsNull() and BRepCheck_Analyzer(cap_shape).IsValid()
    cap_topology = {name: len(entities(cap_shape, kind)) for name, kind in [
        ('solids', TopAbs_SOLID), ('faces', TopAbs_FACE), ('edges', TopAbs_EDGE), ('vertices', TopAbs_VERTEX)]}
    assert cap_topology == dict(solids=1, faces=len(capped['faces']), edges=len(capped['edges']), vertices=len(capped['vertices']))
    assert cap_topology['faces'] == 8
    cap_props = GProp_GProps()
    BRepGProp.VolumeProperties_s(cap_shape, cap_props, 1e-10, True, False)
    expected = float(reference['expectedChangedVolumeMm3'])
    cap_error = abs(cap_props.Mass() - expected)
    assert cap_error < 1e-9
    for root_vertex in roots:
        matches = [v for v in entities(cap_shape, TopAbs_VERTEX)
                   if math.dist(root_vertex['point'], BRep_Tool.Pnt_s(TopoDS.Vertex_s(v)).Coord()) < 1e-9]
        assert len(matches) == 1
        vertex = matches[0]
        assert sum(any(vertex.IsSame(v) for v in entities(e, TopAbs_VERTEX)) for e in entities(cap_shape, TopAbs_EDGE)) == 3
        assert sum(any(vertex.IsSame(v) for v in entities(f, TopAbs_VERTEX)) for f in entities(cap_shape, TopAbs_FACE)) == 3
    for trace in traces:
        matching = [i for i, e in enumerate(capped['edges']) if e['curve'] == trace['curve']]
        assert len(matching) == 1
        uses = [c for f in capped['faces'] for w in [f['outer'], *f['holes']]
                for c in capped['loops'][w]['coedges'] if c['edge'] == matching[0]]
        assert len(uses) == 2
    report['explicitSeedCap'] = dict(topology=cap_topology, volumeMm3=cap_props.Mass(),
        referenceVolumeMm3=expected, volumeErrorMm3=cap_error, originalRootVerticesShared=True,
        automaticMaterialSelectionQualified=bool(cap_meta['automaticMaterialSelectionQualified']), filletQualified=False)
    report['scope'] += '; saved rational cap checked against actual-coefficient quadrature; automatic mode validates the native automatic-selection export and receipts, not arbitrary trimming or fillets'
    report['automaticMode'] = automatic
    report['fileSha256'].update({name: hashlib.sha256((root / name).read_bytes()).hexdigest()
                                for name in ['model.json', 'body.step', 'expected-rational-cap-volume.json']})
if automatic:
    assert report.get('automaticMode') is True
(root / 'independent-rational-partition.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report, indent=2))
