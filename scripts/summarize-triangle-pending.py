#!/usr/bin/env python3
"""Summarize native unresolved UV regions without treating them as defects or proofs."""
import argparse
import collections
import hashlib
import json
import math
from pathlib import Path


def summarize(document):
    rows = {}
    kinds = {item['face']: item['kind'] for item in document.get('faceKinds', [])}
    pending = document['refinement']['pending']
    for item in pending:
        face = item['face']
        row = rows.setdefault(face, {'face': face, 'kind': kinds.get(face, 'unreported'), 'triangles': 0, 'reasons': collections.Counter(),
                                    'uvBounds': [[math.inf, -math.inf], [math.inf, -math.inf]],
                                    'triangleUvAreaSum': 0., 'addresses': []})
        uv, vertices = item['uv'], item['vertices']
        if len(uv) != 3 or len(vertices) != 3 or any(len(p) != 2 or not all(math.isfinite(x) for x in p) for p in uv):
            raise ValueError('Invalid native triangle address')
        row['triangles'] += 1
        row['reasons'][item['reason']] += 1
        for axis in range(2):
            row['uvBounds'][axis][0] = min(row['uvBounds'][axis][0], *(p[axis] for p in uv))
            row['uvBounds'][axis][1] = max(row['uvBounds'][axis][1], *(p[axis] for p in uv))
        a, b, c = uv
        row['triangleUvAreaSum'] += abs((b[0]-a[0])*(c[1]-a[1])-(b[1]-a[1])*(c[0]-a[0])) / 2
        row['addresses'].append({'vertices': vertices, 'uv': uv, 'reason': item['reason']})
    return {'schema': 'cad-native-pending-addresses/1', 'solidQualified': document['solidQualified'],
            'acceptedTriangles': document['refinement']['accepted'], 'pendingTriangles': len(pending),
            'scope': 'Diagnostic UV addresses and numerical UV area sums; not physical areas, defects or geometric certificates.',
            'faces': sorted(rows.values(), key=lambda row: (-row['triangles'], row['face']))}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('input', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    raw = args.input.read_bytes()
    result = summarize(json.loads(raw))
    result['inputSha256'] = hashlib.sha256(raw).hexdigest()
    args.output.write_text(json.dumps(result, indent=2, allow_nan=False) + '\n')
    print(json.dumps({key: result[key] for key in ['acceptedTriangles', 'pendingTriangles', 'solidQualified']}))
