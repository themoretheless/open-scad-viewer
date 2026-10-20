#!/usr/bin/env python3
"""Compare addressed native refinement runs at matching input and work limits."""
import argparse
import hashlib
import importlib.util
import json
from pathlib import Path

spec = importlib.util.spec_from_file_location('pending_summary', Path(__file__).with_name('summarize-triangle-pending.py'))
summary = importlib.util.module_from_spec(spec)
spec.loader.exec_module(summary)


def compare(before, after, allow_subdivision_change=False, allow_additional_work=False):
    for key in ['fixture', 'targetDeviationMm', 'candidate']:
        if before[key] != after[key]:
            raise ValueError('Incomparable input: ' + key)
    limits_a, limits_b = before.get('refinementLimits'), after.get('refinementLimits')
    limits_verified = limits_a is not None and limits_b is not None
    if limits_verified and limits_a != limits_b:
        raise ValueError('Incomparable input: refinementLimits')
    subdivision_changed = before['interiorBisection'] != after['interiorBisection']
    if subdivision_changed and not allow_subdivision_change:
        raise ValueError('Incomparable input: interiorBisection; explicitly allow subdivision changes for a diagnostic comparison')
    additional_changed = before.get('regionalTransitionLimits') != after.get('regionalTransitionLimits')
    if additional_changed and not allow_additional_work:
        raise ValueError('Incomparable input: regionalTransitionLimits; explicitly allow additional work')
    a, b = summary.summarize(before), summary.summarize(after)
    def pending_addresses(report):
        return {(row['face'], tuple(row['vertices']), tuple(tuple(point) for point in row['uv']))
                for row in report['refinement']['pending']}
    addresses_a, addresses_b = pending_addresses(before), pending_addresses(after)

    rows_a, rows_b = ({row['face']: row for row in value['faces']} for value in [a, b])
    faces = []
    for face in sorted(rows_a.keys() | rows_b.keys()):
        old, new = rows_a.get(face, {}), rows_b.get(face, {})
        faces.append({'face': face, 'pendingBefore': old.get('triangles', 0),
                      'pendingAfter': new.get('triangles', 0),
                      'pendingUvAreaBefore': old.get('triangleUvAreaSum', 0),
                      'pendingUvAreaAfter': new.get('triangleUvAreaSum', 0),
                      'pendingUvAreaDelta': new.get('triangleUvAreaSum', 0)-old.get('triangleUvAreaSum', 0)})
    return {'schema': 'cad-refinement-comparison/1',
            'subdivisionChanged': subdivision_changed,
            'refinementLimitsVerified': limits_verified,
            'additionalWorkChanged': additional_changed,
            'regionalLimitsBefore': before.get('regionalTransitionLimits'), 'regionalLimitsAfter': after.get('regionalTransitionLimits'),
            'refinementLimitsStatus': 'matching' if limits_verified else 'unavailable-in-one-or-both-reports',
            'comparisonKind': 'changed-subdivision-diagnostic' if subdivision_changed else 'matching-subdivision-diagnostic',
            'bisectionBefore': before['interiorBisection'], 'bisectionAfter': after['interiorBisection'],
            'acceptedBefore': a['acceptedTriangles'], 'acceptedAfter': b['acceptedTriangles'],
            'pendingBefore': a['pendingTriangles'], 'pendingAfter': b['pendingTriangles'],
            'pendingAddressSubset': addresses_b <= addresses_a,
            'pendingNewAddresses': len(addresses_b-addresses_a),
            'pendingRemovedAddresses': len(addresses_a-addresses_b),
            'solidQualified': b['solidQualified'], 'faces': faces,
            'numericFailuresAfter': after['refinement']['numericFailures'],
            'leafIncidenceClosedAfter': after['refinement']['leafIncidenceClosed'],
            'scope': 'Numerical UV coverage diagnostic. Different leaf counts alone do not establish better geometric coverage.'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('before', type=Path)
    parser.add_argument('after', type=Path)
    parser.add_argument('output', type=Path)
    parser.add_argument('--allow-subdivision-change', action='store_true', help='Label differing interior subdivision as a diagnostic comparison')
    parser.add_argument('--allow-additional-work', action='store_true', help='Explicitly label added regional work budget')
    args = parser.parse_args()
    raw_a, raw_b = args.before.read_bytes(), args.after.read_bytes()
    result = compare(json.loads(raw_a), json.loads(raw_b), args.allow_subdivision_change, args.allow_additional_work)
    result.update(beforeSha256=hashlib.sha256(raw_a).hexdigest(), afterSha256=hashlib.sha256(raw_b).hexdigest())
    args.output.write_text(json.dumps(result, indent=2, allow_nan=False)+'\n')
    print(json.dumps({key: result[key] for key in ['acceptedBefore', 'acceptedAfter', 'pendingBefore', 'pendingAfter', 'solidQualified']}))
