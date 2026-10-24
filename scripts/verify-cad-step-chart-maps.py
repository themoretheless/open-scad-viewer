#!/usr/bin/env python3
"""Algebraic STEP chart checks for identity, U reflection and quadratic Bezier spans.

Unsupported representations fail closed. No sampled surface comparison is used.
"""
import argparse
import copy
from fractions import Fraction as F
import hashlib
import json
import math
from pathlib import Path
from importlib.util import spec_from_file_location, module_from_spec

spec = spec_from_file_location('oriented_incidence', Path(__file__).with_name('verify-cad-step-oriented-incidence.py'))
incidence = module_from_spec(spec)
spec.loader.exec_module(incidence)


def chart(source, target):
    if source == target:
        return 1, 0, 'identity'
    reflected = copy.deepcopy(source)
    reflected['controlPoints'].reverse()
    reflected['weights'].reverse()
    reflected['knotsU'] = [1 - k for k in reversed(source['knotsU'])]
    if source['knotsU'] == [0, 0, 1, 1] and reflected == target:
        return -1, 1, 'exact-U-reflection'
    # Internal multiplicity degree gives independent rational Bezier spans.
    # Matching coefficients proves equality over the entire selected span.
    for index in range(4):
        piece = copy.deepcopy(target)
        if target['degreeU'] != 2 or target['knotsU'] != [0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 4]:
            break
        piece['controlPoints'] = target['controlPoints'][2*index:2*index+3]
        piece['weights'] = target['weights'][2*index:2*index+3]
        piece['knotsU'] = [0, 0, 0, 1, 1, 1]
        if piece == source:
            return 1, index, 'exact-owned-Bezier-span'
    raise ValueError('unsupported or unequal surface chart')


def pcurve_bound(source, target, sign, offset, reverse):
    a, b = copy.deepcopy(source), copy.deepcopy(target)
    if reverse:
        a['controlPoints'].reverse()
        a['weights'].reverse()
        lo, hi = a['knots'][a['degree']], a['knots'][-a['degree']-1]
        a['knots'] = [lo + hi - k for k in reversed(a['knots'])]
    assert {k:v for k,v in a.items() if k != 'controlPoints'} == {k:v for k,v in b.items() if k != 'controlPoints'}, 'curve basis changed'
    assert a['periodic'] is False and all(math.isfinite(w) and w > 0 for w in a['weights'])
    assert len(a['controlPoints']) == len(b['controlPoints']) == len(a['weights'])
    degree = a['degree']
    assert isinstance(degree, int) and 0 < degree < len(a['weights'])
    assert len(a['knots']) == len(a['weights']) + degree + 1
    assert all(math.isfinite(k) for k in a['knots']) and a['knots'] == sorted(a['knots'])
    assert a['knots'][degree] < a['knots'][-degree-1]
    error = F(0)
    for p, q in zip(a['controlPoints'], b['controlPoints']):
        # All owned source trims must remain inside the certified chart domain.
        assert all(math.isfinite(x) and 0 <= x <= 1 for x in p)
        expected = [sign * F(p[0]) + offset, F(p[1])]
        error = max(error, *(abs(x - F(y)) for x,y in zip(expected,q)))
    # Equal positive rational basis => error is a convex combination of
    # coefficient errors, for every parameter, not just selected stations.
    assert error <= F(1, 10**14), 'UV coefficient drift exceeds tolerance'
    return error


def verify(source, target):
    rows = []
    for face_id, face in zip(source['topologyIds']['faces'], source['faces']):
        other = target['faces'][target['topologyIds']['faces'].index(face_id)]
        sign, offset, kind = chart(face['surface'], other['surface'])
        maximum = F(0)
        for loop_index in [face['outer'], *face['holes']]:
            loop_id = source['topologyIds']['loops'][loop_index]
            imported_loop = target['loops'][target['topologyIds']['loops'].index(loop_id)]
            a = {source['topologyIds']['edges'][c['edge']]:c for c in source['loops'][loop_index]['coedges']}
            b = {target['topologyIds']['edges'][c['edge']]:c for c in imported_loop['coedges']}
            assert len(a) == len(source['loops'][loop_index]['coedges']) and len(b) == len(imported_loop['coedges']), 'repeated coedge requires another proof'
            assert a.keys() == b.keys()
            for edge_id, c in a.items():
                d = b[edge_id]
                maximum = max(maximum, pcurve_bound(c['pcurve'], d['pcurve'], sign, offset, c['reversed'] != d['reversed']))
        rows.append({'faceIdentity':face_id, 'surfaceProof':kind, 'uSign':sign, 'uOffset':offset,
                     'maxTrimUvDeviationUpperBound':math.nextafter(float(maximum), math.inf) if maximum else 0,
                     'exactUvDeviationFraction':str(maximum)})
    return rows


def main(directory, output):
    parts = []
    for name in ('bracket','flange','enclosure'):
        raw = (directory / (name+'-step-roundtrip.json')).read_bytes()
        doc = json.loads(raw)
        source = next(b['brep'] for b in doc['bodies'] if b['id']==name)
        imported = next(b['brep'] for b in doc['bodies'] if b['name']=='Retained AP242 STEP')
        incidence_checks = incidence.check(source, imported)
        assert all(incidence_checks.values()), 'oriented topology or exact edge binding changed'
        faces = verify(source, imported)
        # Exercise surface and trim errors separately; both must fail closed.
        for mutation in ('surface','trim'):
            bad = copy.deepcopy(imported)
            if mutation == 'surface': bad['faces'][0]['surface']['controlPoints'][0][0][0] += .01
            else: bad['loops'][0]['coedges'][0]['pcurve']['controlPoints'][0][0] += .01
            try: verify(source,bad)
            except (AssertionError, ValueError): pass
            else: raise AssertionError('mutation accepted: '+mutation)
        parts.append({'part':name,'inputSha256':hashlib.sha256(raw).hexdigest(),'faces':faces,'incidenceChecks':incidence_checks,
                      'surfaceMutationRejected':True,'trimMutationRejected':True})
    report = {'schema':'cad-step-chart-maps/1','passed':True,
              'scope':'Exact surface coefficient equality after supported chart maps over the full owned unit domain; positive-rational trim UV error bound over every parameter. No general NURBS chart matching, whole-wall or tangency certification.',
              'trimUvTolerance':1e-14,'parts':parts}
    with output.open('x') as f: json.dump(report,f,indent=2); f.write('\n')
    print(json.dumps({'passed':True,'parts':len(parts),'faces':sum(len(p['faces']) for p in parts),'maxTrimUvDeviationUpperBound':max(f['maxTrimUvDeviationUpperBound'] for p in parts for f in p['faces'])}))


if __name__ == '__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('directory',type=Path)
    parser.add_argument('output',type=Path)
    args=parser.parse_args()
    main(args.directory,args.output)
