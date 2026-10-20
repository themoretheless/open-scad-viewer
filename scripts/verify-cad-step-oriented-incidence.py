#!/usr/bin/env python3
"""Check oriented 3D boundary incidence, independently of UV chart storage.

This does not certify the surface interior or pcurve/surface correspondence.
The stricter raw representation report remains a separate gate.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
from importlib.util import spec_from_file_location, module_from_spec

spec = spec_from_file_location('identity_bindings', Path(__file__).with_name('verify-cad-ui-step-identities.py'))
identity = module_from_spec(spec)
spec.loader.exec_module(identity)


def cycle(items):
    assert items, 'empty boundary'
    return min(tuple(items[i:] + items[:i]) for i in range(len(items)))


def oriented_incidence(model):
    ids = model['topologyIds']
    # Validate unique identities and all entity references through the raw checker.
    for kind in identity.KINDS:
        identity.bindings(model, kind)
    shells = {}
    for shell_id, shell in zip(ids['shells'], model['shells']):
        faces = {}
        for use in shell['faces']:
            face = model['faces'][use['face']]
            face_id = ids['faces'][use['face']]
            assert face_id not in faces, 'duplicate face use'
            loops = {}
            for index in [face['outer'], *face['holes']]:
                loop_id = ids['loops'][index]
                assert loop_id not in loops, 'duplicate loop use'
                coedges = model['loops'][index]['coedges']
                boundary = [(ids['edges'][c['edge']], c['reversed']) for c in coedges]
                # The shell orientation changes when a surface chart is reflected.
                if use['reversed']:
                    boundary = [(edge, not reversed_) for edge, reversed_ in reversed(boundary)]
                loops[loop_id] = cycle(boundary)
            faces[face_id] = {'outer': ids['loops'][face['outer']],
                              'holes': sorted(ids['loops'][i] for i in face['holes']),
                              'orientedBoundaries': loops}
        shells[shell_id] = {'closed': shell['closed'], 'faces': faces}
    return shells


def check(source, imported):
    return {
        'identitySets': all(set(source['topologyIds'][k]) == set(imported['topologyIds'][k]) for k in identity.KINDS),
        'exactVertices': identity.bindings(source, 'vertices') == identity.bindings(imported, 'vertices'),
        'exactEdges': identity.bindings(source, 'edges') == identity.bindings(imported, 'edges'),
        'bodyShellBindings': identity.bindings(source, 'bodies') == identity.bindings(imported, 'bodies'),
        'orientedFaceLoopEdgeBindings': oriented_incidence(source) == oriented_incidence(imported),
    }


def main(directory, output):
    rows = []
    for name in ('bracket', 'flange', 'enclosure'):
        raw = (directory / f'{name}-step-roundtrip.json').read_bytes()
        doc = json.loads(raw)
        source = next(b['brep'] for b in doc['bodies'] if b['id'] == name)
        imported = next(b['brep'] for b in doc['bodies'] if b['name'] == 'Retained AP242 STEP')
        checks = check(source, imported)
        # A wrong shell orientation must be observable, even with identical IDs.
        changed = copy.deepcopy(imported)
        changed['shells'][0]['faces'][0]['reversed'] = not changed['shells'][0]['faces'][0]['reversed']
        assert not check(source, changed)['orientedFaceLoopEdgeBindings']
        # A coedge direction error must also be observable.
        changed = copy.deepcopy(imported)
        changed['loops'][0]['coedges'][0]['reversed'] = not changed['loops'][0]['coedges'][0]['reversed']
        assert not check(source, changed)['orientedFaceLoopEdgeBindings']
        rows.append({'part': name, 'inputSha256': hashlib.sha256(raw).hexdigest(),
                     'checks': checks, 'passed': all(checks.values()), 'orientationMutationRejected': True,
                     'coedgeMutationRejected': True})
    report = {'schema': 'cad-step-oriented-incidence/1', 'passed': all(r['passed'] for r in rows),
              'scope': 'Exact vertex/edge bindings and oriented face-loop-edge incidence modulo cyclic start and shell orientation. Surface interiors and UV pullbacks are not certified.',
              'surfaceChartEquivalenceQualified': False, 'parts': rows}
    with output.open('x') as stream:
        json.dump(report, stream, indent=2)
        stream.write('\n')
    print(json.dumps(report, indent=2))
    return 0 if report['passed'] else 1


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    raise SystemExit(main(args.directory, args.output))
