#!/usr/bin/env python3
"""Verify identity-to-geometry/incidence bindings in actual browser STEP roundtrips.
Array order and import-created document object IDs are not topology identities.
"""
import copy
import argparse
import json
from pathlib import Path
import sys

KINDS = ('vertices', 'edges', 'loops', 'faces', 'shells', 'bodies')

def bindings(model, kind):
    ids = model['topologyIds']
    def ref(target, index):
        return ids[target][index]
    result = {}
    assert len(ids[kind]) == len(model[kind])
    assert len(set(ids[kind])) == len(ids[kind])
    for identity, entity in zip(ids[kind], model[kind]):
        value = copy.deepcopy(entity)
        if kind == 'edges':
            value['vertices'] = [ref('vertices', i) for i in value['vertices']]
        elif kind == 'loops':
            for coedge in value['coedges']:
                coedge['edge'] = ref('edges', coedge['edge'])
        elif kind == 'faces':
            value['outer'] = ref('loops', value['outer'])
            value['holes'] = [ref('loops', i) for i in value['holes']]
        elif kind == 'shells':
            for face in value['faces']:
                face['face'] = ref('faces', face['face'])
        elif kind == 'bodies':
            value['outerShell'] = ref('shells', value['outerShell'])
            value['innerShells'] = [ref('shells', i) for i in value['innerShells']]
        result[identity] = value
    return result

def main(directory, source_kind="browser"):
    assert source_kind in ("browser", "native-fresh")
    rows = []
    for name in ('bracket', 'flange', 'enclosure'):
        doc = json.loads((directory / f'{name}-step-roundtrip.json').read_text())
        source = next(b for b in doc['bodies'] if b['id'] == name)['brep']
        imported = next(b for b in doc['bodies'] if b['name'] == 'Retained AP242 STEP')['brep']
        checks = {kind: bindings(source, kind) == bindings(imported, kind) for kind in KINDS}
        identity_sets = {kind: set(source['topologyIds'][kind]) == set(imported['topologyIds'][kind]) for kind in KINDS}
        rows.append(dict(part=name, topologyIdentitySets=identity_sets, exactRepresentationBindings=checks,
                         identitiesPreserved=all(identity_sets.values()), passed=all(checks.values()),
                         arrayOrderEqual={k: source['topologyIds'][k] == imported['topologyIds'][k] for k in KINDS}))
    report = dict(schema='cad-ui-step-identity-bindings/1' if source_kind=='browser' else 'cad-fresh-step-identity-bindings/1', source=source_kind, passed=all(r['passed'] for r in rows),
                  scope=('Identity sets and exact representation bindings for actual UI roundtrips. ' if source_kind=='browser' else 'Identity sets and exact representation bindings for fresh native STEP roundtrips; no browser action is covered. ')+'Equal identity sets do not prove orientation or surface reparameterization equivalence. Document body UUID and history lineage are separate.', parts=rows)
    (directory / 'identity-report.json').write_text(json.dumps(report, indent=2) + '\n')
    print(json.dumps(report, indent=2))
    return 0 if report['passed'] else 1

if __name__ == '__main__':
    parser=argparse.ArgumentParser()
    parser.add_argument('directory',type=Path)
    parser.add_argument('--source',choices=('browser','native-fresh'),default='browser')
    args=parser.parse_args()
    sys.exit(main(args.directory,args.source))
